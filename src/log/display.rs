use core::mem::transmute;

use super::{Message, Verbosity};

/// A wrapper around a [`Message`] that implements [`core::fmt::Display`]. That implementation uses
/// colors to make the message more readable in a terminal.
#[repr(transparent)]
pub struct WithAnsiColors<'a>(Message<'a>);

impl<'a> WithAnsiColors<'a> {
    /// Creates a new [`WithAnsiColors`] wrapper around the provided message.
    #[inline]
    pub fn wrap(message: &'a Message<'a>) -> &'a Self {
        unsafe { transmute(message) }
    }
}

impl<'a> core::fmt::Display for WithAnsiColors<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (prefix, suffix) = match self.0.verbosity {
            Verbosity::Error => ("  \x1B[1;31merror\x1B[0m  \x1B[31m", "\x1B[0m"),
            Verbosity::Warn => ("   \x1B[1;33mwarn\x1B[0m  \x1B[33m", "\x1B[0m"),
            Verbosity::Info => ("   \x1B[1;34minfo\x1B[0m  \x1B[0m", ""),
            Verbosity::Trace => ("  \x1B[1;30mtrace\x1B[0m  \x1B[90m", "\x1b[0m"),
        };

        f.write_str(prefix)?;
        core::fmt::write(Writer::wrap(f), self.0.message)?;
        f.write_str(suffix)?;

        Ok(())
    }
}
/// A writer implementation that replaces newlines with a newlinew followed by the
/// correct number of spaces to align the text with the prefix.
#[repr(transparent)]
struct Writer<'w>(core::fmt::Formatter<'w>);

impl<'w> Writer<'w> {
    /// Creates a new [`Writer`] wrapper around the provided formatter.
    #[inline]
    pub fn wrap<'a>(f: &'a mut core::fmt::Formatter<'w>) -> &'a mut Self {
        unsafe { transmute(f) }
    }
}

impl<'w> core::fmt::Write for Writer<'w> {
    fn write_str(&mut self, mut s: &str) -> core::fmt::Result {
        loop {
            match s.find('\n') {
                Some(newline) => {
                    let (start, rest) = s.split_at(newline + 1);
                    self.0.write_str(start)?;
                    self.0.write_str("         ")?;
                    s = rest;
                }
                None => {
                    self.0.write_str(s)?;
                    return Ok(());
                }
            }
        }
    }
}
