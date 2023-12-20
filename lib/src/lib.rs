#![no_std]

use loose_enum::loose_enum;

mod sysno;
pub use self::sysno::*;

mod syscalls;
pub use self::syscalls::*;

mod sysresult;
pub use self::sysresult::*;

/// The ID of a process.
pub type ProcessId = usize;

/// A condition that a process can wait on.
#[repr(C)]
#[derive(Clone, Copy)]
pub union WakeUp {
    /// The tag of this [`WakeUp`] variant.
    ///
    /// All other variants must have a field at offset 0 with this type.
    pub tag: WakeUpTag,
    /// Indicates that the process is waiting for a byte of data to be available on the first
    /// PS/2 port.
    pub ps2_one: WakeUpPS2,
    /// Indicates that the process is waiting for a byte of data to be available on the second
    /// PS/2 port.
    pub ps2_two: WakeUpPS2,
}

impl WakeUp {
    /// Returns the tag of this [`WakeUp`] variant.
    #[inline]
    pub fn tag(&self) -> WakeUpTag {
        unsafe { self.tag }
    }
}

loose_enum! {
    /// A tag that describes which condition a [`WakeUp`] is waiting on.
    pub struct WakeUpTag: u8 {
        /// The process is waiting for a byte of data to be available on the first PS/2 port.
        const PS2_ONE = 0;
        /// The process is waiting for a byte of data to be available on the second PS/2 port.
        const PS2_TWO = 1;
    }
}

/// A variant of [`WakeUp`]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct WakeUpPS2 {
    /// The tag of this [`WakeUp`] variant.
    ///
    /// This must be either [`WakeUpTag::PS2_ONE`] or [`WakeUpTag::PS2_TWO`].
    pub tag: WakeUpTag,
    /// A pointer to the byte of data that was read from the PS/2 port.
    pub data: u8,
}

unsafe impl Send for WakeUpPS2 {}
unsafe impl Sync for WakeUpPS2 {}

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
