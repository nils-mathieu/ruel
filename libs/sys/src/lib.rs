#![no_std]

use bitflags::bitflags;
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

/// A buffer that can hold PS/2 scan-codes.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PS2Buffer {
    /// The number of bytes that have been written to the buffer.
    ///
    /// # Remarks
    ///
    /// It's possible for `length` to be larger than `SIZE`. This can be used to detect whether
    /// some bytes have been dropped since the last time the buffer was read.
    pub length: u8,
    /// The buffer.
    pub buffer: [u8; Self::SIZE],
}

impl PS2Buffer {
    /// The maximum number of bytes that can be received by the program during a single quantum.
    pub const SIZE: usize = 7;

    /// An empty [`PS2Buffer`].
    pub const EMPTY: Self = Self {
        length: 0,
        buffer: [0; Self::SIZE],
    };
}

bitflags! {
    /// Some flags used to configure a running process instance.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct ProcessConfig: usize {
        /// Whether the process wants to block on potentially blocking system calls.
        ///
        /// When this flag is set, system calls that would block the process will return instantly
        /// without blocking.
        ///
        /// Otherwise, those system calls will block the process until the condition is met.
        const DONT_BLOCK = 1 << 0;
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

/// Information about an available framebuffer.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Framebuffer {
    /// The virtual address of the framebuffer.
    pub address: *mut u8,
    /// The pitch of the framebuffer.
    ///
    /// This is the number of bytes between the start of one row of pixels and the start of the
    /// next row of pixels.
    pub bytes_per_lines: usize,
    /// The width of the framebuffer.
    pub width: u32,
    /// The height of the framebuffer.
    pub height: u32,
    /// The format of the framebuffer.
    pub format: FramebufferFormat,
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

impl Framebuffer {
    /// Returns the size of the framebuffer, in bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.bytes_per_lines * self.height as usize
    }
}

loose_enum! {
    /// The video mode of a [`Framebuffer`].
    pub struct FramebufferFormat: u32 {
        /// Each pixel of the framebuffer is represented by three bytes; one for each color channel
        /// in the order red, green, and then blue.
        const RGB24 = 0;
        /// Each pixel of the framebuffer is represented by four bytes; one for each color channel
        /// in the order red, green, blue.
        ///
        /// The most significant byte is unused.
        const RGB32 = 1;
        /// Each pixel of the framebuffer is represented by three bytes; one for each color channel
        /// in the order blue, green, and then red.
        const BGR24 = 2;
        /// Each pixel of the framebuffer is represented by four bytes; one for each color channel
        /// in the order blue, green, red.
        ///
        /// The first significant byte is unused.
        const BGR32 = 3;
    }
}

impl FramebufferFormat {
    /// Retrurns the number of bytes per pixel of the framebuffer.
    ///
    /// # Remarks
    ///
    /// If the format is not known, this function returns 0.
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::RGB24 | Self::BGR24 => 3,
            Self::RGB32 | Self::BGR32 => 4,
            _ => 0,
        }
    }
}
