use core::arch::asm;

use x86_64::{wrmsr, Efer, Ring, LSTAR, STAR};

use crate::log;

///
#[naked]
unsafe extern "C" fn syscall_handler() {
    unsafe {
        asm!(
            "
            sysretq
            ",
            options(noreturn)
        )
    }
}

/// Initialize the system call handler.
#[allow(clippy::assertions_on_constants)]
pub fn init() {
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
