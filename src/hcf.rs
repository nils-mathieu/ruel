//! This module implements the panic handler for the whole system. See [`hcf`].

use core::arch::asm;
use core::panic::PanicInfo;

use crate::log;

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
fn panic_routine(info: &PanicInfo) -> ! {
    let message: &dyn core::fmt::Display = match info.message() {
        Some(m) => m,
        None => &"no further information",
    };

    let location: &dyn core::fmt::Display = match info.location() {
        Some(l) => l,
        None => &"unknown location",
    };

    log::error!(
        "\
        The kernel panicked.\n\
        \n\
        This is a serious bug in the kernel. Please report it by\n\
        opening an issue on the GitHub repository.\n\
        \n\
        https://github.com/nils-mathieu/ruel/issues/new\n\
        \n\
        Message: {message}\n\
        Location: {location}\
        ",
    );
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
