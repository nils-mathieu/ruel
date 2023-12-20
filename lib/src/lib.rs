#![no_std]

use loose_enum::loose_enum;

mod sysno;
pub use self::sysno::*;

mod syscalls;
pub use self::syscalls::*;

mod sysresult;
pub use self::sysresult::*;

loose_enum! {
    /// A boolean value.
    ///
    /// # Why is this needded?
    ///
    /// The layout of the standard `bool` type requires that only the bit patterns `0` and `1` are
    /// valid. This makes the type unsuitable for use in any kind of FFI, as it's not guaranteed
    /// that other languages will follow the same convention.
    ///
    /// This type is guaranteed to have the same layout as a `u8`, and thus can be used in FFI.
    pub struct Bool: u8 {
        /// The `false` value.
        const FALSE = 0;
        /// The `true` value.
        const TRUE = 1;
    }
}

impl Bool {
    /// Creates a new [`Bool`] from the provided boolean value.
    #[inline]
    pub fn as_bool(self) -> bool {
        self != Self::FALSE
    }
}

impl From<bool> for Bool {
    #[inline]
    fn from(b: bool) -> Self {
        if b {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }
}

impl From<Bool> for bool {
    #[inline]
    fn from(b: Bool) -> Self {
        b.as_bool()
    }
}

/// The ID of a process.
pub type ProcessId = usize;

/// A condition that a process can wait on.
#[repr(C)]
#[derive(Clone, Copy)]
pub union WakeUp {
    /// The tag of the [`WakeUp`], indicating which condition is being waited on.
    pub tag: WakeUpTag,
    /// Indicates that the process is waiting for a byte of data to be available from the PS/2
    /// keyboard.
    pub ps2_keyboard: WakeUpPS2,
}

impl WakeUp {
    /// Returns the tag of this [`WakeUp` variant.
    #[inline]
    pub fn tag(&self) -> WakeUpTag {
        // SAFETY:
        //  The tag is in all variants of the union.
        unsafe { self.tag }
    }
}

loose_enum! {
    /// A tag that describes which condition a [`WakeUp`] is waiting on.
    pub struct WakeUpTag: u8 {
        /// The process is waiting for a byte of data to be available on the first PS/2 port.
        const PS2_KEYBOARD = 0;
    }
}

/// A variant of [`WakeUp`]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct WakeUpPS2 {
    /// The tag of the [`WakeUp`] variant.
    ///
    /// For this variant, this is always [`WakeUpTag::PS2_KEYBOARD`].
    pub tag: WakeUpTag,
    /// A pointer to the byte of data that was read from the PS/2 port.
    pub data: [u8; Self::MAX_DATA_LENGTH],
    /// The number of bytes that were read from the PS/2 port.
    pub count: u8,
}

impl WakeUpPS2 {
    /// The maximum nubmer of bytes that can be read from the PS/2 port during a single
    /// quantum.
    pub const MAX_DATA_LENGTH: usize = 5;
}

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
