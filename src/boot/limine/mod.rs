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
    if !raw::base_revision_supported() {
        // TODO: Print an error message.
        hcf();
    }

    // SAFETY:
    //  We're at the beginning of the entry point function executed by the bootloader. The
    //  bootloader reclaimable memory region is still intact.
    let token = unsafe { req::Token::get() };

    if token.entry_point().is_none() {
        // TODO: Print an error message.
        hcf();
    }

    todo!();
}
