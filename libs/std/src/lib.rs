//! The standard userspace library for Rust applications targeting Ruel OS.

#![no_std]

pub use sys::SysResult;

/// The result type of the crate.
pub type Result<T> = core::result::Result<T, SysResult>;

#[cfg(feature = "framebuffer")]
pub mod framebuffer;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "sleep")]
pub mod sleep;

pub extern crate sys;

pub use sys::despawn_self;
