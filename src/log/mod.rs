//! The logging module of the kernel.
//!
//! This moduel provides a simple logging interface for the kernel, and allows printing messages
//! through the serial port and to the screen (if it is available).

use core::fmt::Arguments;

mod display;
pub use self::display::*;

#[cfg(feature = "debug-serial")]
mod serial;

/// A message that the kernel can print.
pub struct Message<'a> {
    /// The message itself.
    pub message: Arguments<'a>,
    /// The verbosity level of the message.
    pub verbosity: Verbosity,
    /// The file in which the message was generated.
    pub file: &'static str,
    /// The line at which the message was generated.
    pub line: u32,
    /// The column at which the message was generated.
    pub column: u32,
}

impl<'a> Message<'a> {
    /// Returns a [`WithAnsiColors`] wrapper around this message.
    #[inline]
    pub fn with_ansi_colors(&self) -> &WithAnsiColors {
        WithAnsiColors::wrap(self)
    }

    /// Logs this message.
    pub fn log(self) {
        #[cfg(feature = "debug-serial")]
        let _ = core::fmt::write(
            &mut serial::Serial::get(),
            format_args!("{}\n", self.with_ansi_colors()),
        );
    }
}

/// The verbosity level of a [`Message`].
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
    Error,
    /// Indicates a warning that occurred in the kernel.
    ///
    /// Those are generally errors that the kernel managed to recover from.
    Warn,
    /// Notifies the user of something that happened in the kernel. It's not an error, but it's
    /// good to know.
    Info,
    /// Provides verbose information about the kernel's current state and execution.
    Trace,
}

/// Creates a new [`Message`] instance with the provided verbosity level and message.
///
/// The provenance information associated with the message is automatically filled with the
/// location at which this macro is invoked.
pub macro message($verbosity:expr, $($arg:tt)*) {
    $crate::log::Message {
        message: format_args!($($arg)*),
        verbosity: $verbosity,
        file: file!(),
        line: line!(),
        column: column!(),
    }
}

/// Logs a message with the provided verbosity level.
pub macro log($verbosity:expr, $($arg:tt)*) {
    $crate::log::message!($verbosity, $($arg)*).log();
}

/// Logs an error message.
pub macro error($($arg:tt)*) {
    $crate::log::log!($crate::log::Verbosity::Error, $($arg)*);
}

/// Logs a warning message.
pub macro warn($($arg:tt)*) {
    $crate::log::log!($crate::log::Verbosity::Warn, $($arg)*);
}

/// Logs an information message.
pub macro info($($arg:tt)*) {
    $crate::log::log!($crate::log::Verbosity::Info, $($arg)*);
}

/// Logs a trace message.
pub macro trace($($arg:tt)*) {
    $crate::log::log!($crate::log::Verbosity::Trace, $($arg)*);
}
