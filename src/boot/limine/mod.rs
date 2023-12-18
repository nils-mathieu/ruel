//! Defines the entry point of the kernel when it is booted by a Limine-compliant bootloader.
//!
//! This is a simple implementation of the [Limine boot protocol][PROTOCOL].
//!
//! [PROTOCOL]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md
//!
//! # Version
//!
//! The version 6 of the protocol is implemented.

use crate::hcf::hcf;
use crate::log;

mod raw;
mod req;

/// The entry point of the kernel when it is booted by a Limine-compliant bootloader.
///
/// # Safety
///
/// - This function expects to be called by a Limine-compliant bootloader, meaning that the
///   machine must currently be in the state described in the [Entry Machine State] section
///   of the protocol.
///
/// - It must only be called once.
///
/// [Entry Machine State]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#entry-memory-layout
unsafe extern "C" fn main() -> ! {
    log::info!("Booting Ruel from the Limine entry point...");

    // =============================================================================================
    // Sanity Checks
    // =============================================================================================
    log::trace!("Performing some sanity checks...");

    if !raw::base_revision_supported() {
        log::error!(
            "\
            The bootloader does not support the base revision expected by the kernel.\n\
            This happens because you're bootloader is outdated.\n\
            \n\
            Please update your bootloader.\
            ",
        );
        hcf();
    }

    // SAFETY:
    //  We're at the beginning of the entry point function executed by the bootloader. The
    //  bootloader reclaimable memory region is still intact.
    let token = unsafe { req::Token::get() };

    if token.entry_point().is_none() {
        log::warn!(
            "\
            The bootloader did not respond to the `limine_entry_point` request of the kernel.\n\
            This is a bug in the bootloader; the protocol requires it to respond to this\n\
            request.\
            ",
        );
    }

    todo!();
}
