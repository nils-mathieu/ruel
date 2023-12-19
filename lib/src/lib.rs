#![no_std]

mod sysno;
pub use self::sysno::*;

mod syscalls;
pub use self::syscalls::*;

mod sysresult;
pub use self::sysresult::*;

/// The ID of a process.
pub type ProcessId = usize;

/// A slice of memory in the address space of a process.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slice {
    /// The address of the pointed memory.
    pub address: *const u8,
    /// The number of bytes in the slice.
    pub length: usize,
}

impl Slice {
    /// Constructs a [`Slice`] using the provided address and length.
    ///
    /// # Safety
    ///
    /// This [`Slice`] instance must reference valid memory that must remain borrowed for the
    /// lifetime `'a`.
    #[inline]
    pub unsafe fn as_slice<'a>(self) -> &'a [u8] {
        core::slice::from_raw_parts(self.address, self.length)
    }

    /// Constructs a mutable [`Slice`] using the provided address and length.
    ///
    /// # Safety
    ///
    /// This [`Slice`] instance must reference valid memory that must remain exclusively borrowed
    /// for the lifetime `'a`.
    #[inline]
    pub unsafe fn as_slice_mut<'a>(self) -> &'a mut [u8] {
        core::slice::from_raw_parts_mut(self.address as *mut u8, self.length)
    }
}

unsafe impl Send for Slice {}
unsafe impl Sync for Slice {}

impl<T: ?Sized + AsRef<[u8]>> From<&T> for Slice {
    #[inline]
    fn from(slice: &T) -> Self {
        let s = slice.as_ref();

        Self {
            address: s.as_ptr(),
            length: s.len(),
        }
    }
}

/// The verbosity level of a message logged through the logging system of the kernel.
///
/// # Remarks
///
/// The ordering of the variants is important, as it is used to determine whether a message should
/// be printed or not.
///
/// Messages that are *more verbose* are *greater* than messages that are *less verbose* (e.g.
/// `Trace` > `Error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verbosity {
    /// Indicates an error that occurred in the kernel.
    ///
    /// Those errors are generally not recoverable, and the kernel will likely has to halt.
    Error = 0,
    /// Indicates a warning that occurred in the kernel.
    ///
    /// Those are generally errors that the kernel managed to recover from.
    Warn = 1,
    /// Notifies the user of something that happened in the kernel. It's not an error, but it's
    /// good to know.
    Info = 2,
    /// Provides verbose information about the kernel's current state and execution.
    Trace = 3,
}

impl Verbosity {
    /// Creates a new [`Verbosity`] from the provided raw value, if it is valid.
    ///
    /// # Errors
    ///
    /// If `n` is larger than 4, this function returns `None`.
    #[inline]
    pub fn from_raw(n: usize) -> Option<Self> {
        match n {
            0 => Some(Self::Error),
            1 => Some(Self::Warn),
            2 => Some(Self::Info),
            3 => Some(Self::Trace),
            _ => None,
        }
    }
}
