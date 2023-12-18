use core::arch::asm;

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

    // Intel processors normally use **SYSENTER** and **SYSEXIT** instructions to perform system
    // calls. However, Intel also provide a way to use the **SYSCALL** and **SYSRET** instructions
    // instead. This is what we're going to use, because that allows us to be compatible with AMD
    // processors.
    enable_syscalls();

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
    const SYSRET_BASE: u16 = USER_CODE_SELECTOR - 2 * 8;
    assert!(USER_CODE_SELECTOR & 0b11 == 0b11);
    assert!(USER_DATA_SELECTOR & 0b11 == 0b11);
    assert!(USER_CODE_SELECTOR == SYSRET_BASE + 16);
    assert!(USER_DATA_SELECTOR == SYSRET_BASE + 8);

    // This constant specifies the segment selectors that will be loaded when the **SYSCALL**
    // instruction is loaded.
    // The CS register will be set to this value, and the SS register will be set to this value
    // plus 8.
    const SYSCALL_BASE: u16 = KERNEL_CODE_SELECTOR;
    assert!(KERNEL_CODE_SELECTOR & 0b11 == 0b00);
    assert!(KERNEL_DATA_SELECTOR & 0b11 == 0b00);
    assert!(KERNEL_CODE_SELECTOR == SYSCALL_BASE);
    assert!(KERNEL_DATA_SELECTOR == SYSCALL_BASE + 8);

    register_syscall_segments(SYSCALL_BASE, SYSRET_BASE);
}

/// Enables the **SYSCALL** and **SYSRET** instructions by writing to the Extended Feature
/// Enable Register (EFER).
fn enable_syscalls() {
    const EFER: u32 = 0xC000_0080;
    const SYSCALL_ENABLE: u64 = 1;

    unsafe {
        asm!(
            "
            rdmsr
            or eax, {}
            wrmsr
            ",
            const SYSCALL_ENABLE,
            in("ecx") EFER,
            lateout("eax") _,
            lateout("edx") _,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Registers the system call routine to be called when the **SYSCALL** instruction is executed.
fn register_syscall_routine(routine: usize) {
    const LSTAR: u32 = 0xC000_0082;

    unsafe {
        asm!(
            "wrmsr",
            in("ecx") LSTAR,
            in("eax") routine as u32,
            in("edx") (routine >> 32) as u32,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Register the code segment and data segments that should be loaded when the syscall/sysret
/// instructions are executed.
fn register_syscall_segments(syscall: u16, sysret: u16) {
    const STAR: u32 = 0xC000_0081;

    unsafe {
        asm!(
            "wrmsr",
            in("ecx") STAR,
            in("eax") 0,
            in("edx") syscall as u32 | (sysret as u32) << 16,
            options(nomem, nostack, preserves_flags),
        );
    }
}
