use core::arch::asm;

use ruel_sys::SysResult;
use x86_64::{wrmsr, Efer, Ring, LSTAR, STAR};

use crate::global::{GlobalToken, KERNEL_STACK_TOP};
use crate::log;

mod handlers;

/// The type of a system call handler.
type SystemCallFn = unsafe extern "C" fn(usize, usize, usize, usize, usize, usize) -> SysResult;

/// The total number of system calls.
const SYSTEM_CALL_COUNT: usize = 6;

/// A lookup table of system call handlers.
///
/// The functions specified in this table are called in the [`system_call`] function. Attempting
/// to perform a system call with an invalid index should always return the
/// [`SysResult::INVALID_VALUE`] error.
static SYSTEM_CALLS: [SystemCallFn; SYSTEM_CALL_COUNT] = [
    handlers::despawn_process,
    handlers::sleep,
    handlers::acquire_framebuffers,
    handlers::release_framebuffers,
    handlers::read_value,
    handlers::kernel_log,
];

/// The function that is called when a userspace program executes the `syscall` instruction.
///
/// # Arguments
///
/// The meaning of arguments to this function depend on the value of the `rax` register.
///
/// Arguments are passed in the following registers:
///
/// - `rdi`
/// - `rsi`
/// - `rdx`
/// - `r10`
/// - `r8`
/// - `r9`
///
/// These registers have been choosen to match the calling convention used by the C language.
/// One exception to this is the `rcx` register, which has been replaced by `r10`. This is because
/// the `syscall` instruction uses the `rcx` register to store the return address, and we need to
/// save it on the stack before calling the system call handler. For this reason, we use `r10` to
/// pass the fourth argument.
///
/// # Register Preservation
///
/// Registers in bold are the one that are different from the C calling convention.
///
/// **SCRATCH REGISTERS:** rax, rdi, rsi, rdx, ~**rcx**~, r8, r9, r10, r11, **r12**.
///
/// **PRESERVED REGISTERS:** rbx, rsp, rbp, ~**r12**~, r13, r14, r15
///
/// # Returns
///
/// The return value of this function is passed in the `rax` register.
///
/// # Safety
///
/// When called from a system call, this function always safe to call. The state of the kernel
/// when that happens must be valid and the global state must be initialized.
///
/// # Clobbered Registers
///
/// The same rules as for the C calling convention apply.
#[naked]
unsafe extern "C" fn syscall_handler() {
    unsafe {
        // The `syscall` instruction invoked by the userland program puts the return address in
        // the `rcx` register. We need to save it on the stack before clobbering all the registers
        // by calling the system call handler.
        //
        // Note that system calls must not touch the stack of the caller, as it might be invalid
        // or broken. Instead, we need to use our own stack. The stack pointer of the caller is
        // saved on the kernel stack, and will be restored before returning with `sysretq`.
        //
        // We're calling a C function, which writes the return value in the `rax` register. Our
        // system calls also return the value in `rax`, so we don't need to do anything more than
        // calling the function.
        asm!(
            r#"
            cmp rax, {syscall_count}
            jae 2f

            mov r12, [{kernel_stack_top}]
            sub r12, 8
            and r12, -8
            mov [r12], rsp
            mov rsp, r12

            push rbp
            mov rbp, rsp
            push rcx

            mov rcx, r10
            call [{system_calls} + 8 * rax]

            pop rcx
            pop rbp
            pop rsp
            sysretq

        2:
            mov rax, {invalid_syscall_number}
            sysretq
            "#,
            kernel_stack_top = sym KERNEL_STACK_TOP,
            syscall_count = const SYSTEM_CALL_COUNT,
            system_calls = sym SYSTEM_CALLS,
            invalid_syscall_number = const SysResult::INVALID_VALUE.as_raw(),
            options(noreturn),
        )
    }
}

/// Initialize the system call handler.
#[allow(clippy::assertions_on_constants)]
pub fn init() {
    assert!(GlobalToken::is_initialized());

    log::trace!("Initializing the system call handler...");

    // Specify the address of the system call handler.
    // The process will jump to this virtual address when the **SYSCALL** instruction is executed.
    register_syscall_routine(syscall_handler as usize);

    // Specify the code segment and data segment to use when executing the **SYSCALL** and
    // **SYSRET** instructions.
    use crate::cpu::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};
    use crate::cpu::gdt::{USER_CODE_SELECTOR, USER_DATA_SELECTOR};

    // This constant specifies the segment selectors that will be loaded when the **SYSRET**
    // instruction is loaded.
    // The CS register will be set to this value plus 16. And the SS register will be set to this
    // value plus 8.
    const SYSRET_BASE: u16 = USER_CODE_SELECTOR.bits() - 2 * 8;
    assert!(USER_CODE_SELECTOR.privilege() == Ring::Three);
    assert!(USER_DATA_SELECTOR.privilege() == Ring::Three);
    assert!(USER_CODE_SELECTOR.bits() == SYSRET_BASE + 16);
    assert!(USER_DATA_SELECTOR.bits() == SYSRET_BASE + 8);

    // This constant specifies the segment selectors that will be loaded when the **SYSCALL**
    // instruction is loaded.
    // The CS register will be set to this value, and the SS register will be set to this value
    // plus 8.
    const SYSCALL_BASE: u16 = KERNEL_CODE_SELECTOR.bits();
    assert!(KERNEL_CODE_SELECTOR.privilege() == Ring::Zero);
    assert!(KERNEL_DATA_SELECTOR.privilege() == Ring::Zero);
    assert!(KERNEL_CODE_SELECTOR.bits() == SYSCALL_BASE);
    assert!(KERNEL_DATA_SELECTOR.bits() == SYSCALL_BASE + 8);

    register_syscall_segments(SYSCALL_BASE, SYSRET_BASE);

    // Intel processors normally use **SYSENTER** and **SYSEXIT** instructions to perform system
    // calls. However, Intel also provide a way to use the **SYSCALL** and **SYSRET** instructions
    // instead. This is what we're going to use, because that allows us to be compatible with AMD
    // processors.
    enable_syscalls();
}

/// Enables the **SYSCALL** and **SYSRET** instructions by writing to the Extended Feature
/// Enable Register (EFER).
#[inline]
fn enable_syscalls() {
    Efer::read().union(Efer::SYSCALL).write();
}

/// Registers the system call routine to be called when the **SYSCALL** instruction is executed.
#[inline]
fn register_syscall_routine(routine: usize) {
    unsafe { wrmsr(LSTAR, routine as u64) }
}

/// Register the code segment and data segments that should be loaded when the syscall/sysret
/// instructions are executed.
#[inline]
fn register_syscall_segments(syscall: u16, sysret: u16) {
    unsafe { wrmsr(STAR, (syscall as u64) << 32 | (sysret as u64) << 48) }
}
