//! This module defines the different entry points of the kernel.
//!
//! # Remarks
//!
//! The kernel may be bootstrapped by multiple different bootloaders (each one is gated behind a
//! feature flag). Each bootloader may have its own entry point, and thus its own `entry_point`
//! function.
//!
//! # Support
//!
//! At the moment, only the [Limine] boot protocol is supported (see the [`limine`] module).
//!
//! [Limine]: https://github.com/limine-bootloader/limine

#[cfg(feature = "boot-limine")]
mod limine;

mod init_process;

/// Prints an helpful message and halts the CPU.
fn oom() -> ! {
    crate::log::error!(
        "\
        The system ran out of memory while booting up. This is likely due to a bug in the\n\
        kernel, but your system might just be missing the memory required to boot.\n\
        \n\
        If you believe that this is an error, please file an issue on the GitHub repository!\n\
        \n\
        https://github.com/nils-mathieu/ruel/issues/new\
        "
    );
    crate::hcf::die();
}

/// Handles a mapping error.
fn handle_mapping_error(err: crate::cpu::paging::MappingError) -> ! {
    match err {
        crate::cpu::paging::MappingError::AlreadyMapped => {
            panic!("attempted to map a page that is already mapped");
        }
        crate::cpu::paging::MappingError::OutOfMemory => oom(),
    }
}
