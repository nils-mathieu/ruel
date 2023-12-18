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
