//! The standard userspace library for Rust applications targeting Ruel OS.

#![no_std]

pub use sys::SysResult;

/// The result type of the crate.
pub type Result<T> = core::result::Result<T, SysResult>;

#[cfg(feature = "clock")]
pub mod clock;
#[cfg(feature = "framebuffer")]
pub mod framebuffer;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "sleep")]
pub mod sleep;

pub extern crate sys;

/// Despawns the current process.
#[inline]
pub fn despawn() -> ! {
    unsafe {
        let _ = sys::despawn_process(sys::ProcessId::MAX);
        core::hint::unreachable_unchecked();
    }
}
