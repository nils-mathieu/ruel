use sys::SysResult;

use crate::Result;

/// A buffer responsible for storing PS/2 scan-codes sent to the kernel.
pub struct PS2Buffer(sys::PS2Buffer);

impl PS2Buffer {
    /// An empty [`PS2Buffer`] instance.
    pub const EMPTY: Self = Self(sys::PS2Buffer::EMPTY);

    /// The maximum number of bytes that can be received by the program during a single quantum.
    pub const SIZE: usize = sys::PS2Buffer::SIZE;

    /// Returns the total number of bytes that have been received by the application since the last
    /// time the buffer was read.
    ///
    /// If more than [`PS2Buffer::SIZE`] bytes have been received, then it means that some bytes
    /// have been dropped.
    #[inline]
    pub fn total_length(&self) -> usize {
        self.0.length as usize
    }

    /// Returns the number of bytes that have been dropped since the last time the buffer was read.
    ///
    /// If no bytes have been dropped, then this function returns `0`.
    #[inline]
    pub fn dropped_bytes(&self) -> usize {
        self.total_length().saturating_sub(PS2Buffer::SIZE)
    }

    /// Returns whether some bytes have been dropped since the last time the buffer was read.
    #[inline]
    pub fn has_dropped_bytes(&self) -> bool {
        self.total_length() > PS2Buffer::SIZE
    }

    /// Returns the scan-codes that have been received by the application since the last time
    /// the buffer was read.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        let len = self.total_length().min(PS2Buffer::SIZE);
        unsafe { core::slice::from_raw_parts(self.0.buffer.as_ptr(), len) }
    }

    /// Returns an iterator over the scan-codes that have been received by the application since
    /// the last time the buffer was read.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.as_slice().iter().copied()
    }

    /// Returns whether no bytes have been received by the application since the last time the
    /// buffer was read.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total_length() == 0
    }

    /// Reads more scan-codes from the PS/2 controller into the buffer.
    ///
    /// See [`sys::read_ps2`] for more information.
    #[inline]
    pub fn read(&mut self) -> Result<()> {
        match sys::read_ps2(&mut self.0) {
            SysResult::SUCCESS => Ok(()),
            err => Err(err),
        }
    }
}
