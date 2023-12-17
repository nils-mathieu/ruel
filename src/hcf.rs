//! This module implements the panic handler for the whole system. See [`hcf`].

use core::arch::asm;
use core::panic::PanicInfo;

/// Halts and catches fire.
///
/// # Panics inside the kernel
///
/// Kernel code should generally never panic under any circumstances. Getting to this function is
/// a serious bug.
///
/// This function attempts to rely on the least amount of code within the kernel as possible, as it
/// is unable to know which parts are safe to use, and which are not.
#[panic_handler]
fn panic_routine(_info: &PanicInfo) -> ! {
    // TODO: Print the panic information through the serial port and to the screen.
    hcf();
}

/// Stops the CPU from receiving interrupts and halts forever.
pub fn hcf() -> ! {
    unsafe {
        asm!(
            "
            cli
            2:
            hlt
            jmp 2b
            ",
            options(noreturn, nomem, nostack, preserves_flags)
        );
    }
}
