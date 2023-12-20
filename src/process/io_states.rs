use ruel_sys::PS2Buffer;

use crate::utility::ArrayVecDequeue;

/// A ring buffer.
pub struct RingBuffer<T, const N: usize> {
    /// The buffer.
    vec: ArrayVecDequeue<T, N>,
    /// Whether some values have been dropped since the last time the buffer was cleared.
    total_len: u8,
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Creates a new [`RingBuffer`] instance.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            vec: ArrayVecDequeue::new_array(),
            total_len: 0,
        }
    }

    /// Clears the ring buffer.
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear();
        self.total_len = 0;
    }

    /// Returns a new [`RingBuffer`] instance with the given capacity.
    #[inline]
    pub fn push(&mut self, item: T) {
        self.vec.push_overwrite(item);
        self.total_len = self.total_len.saturating_add(1);
    }

    /// Returns whether the ring buffer dropped some items.
    #[inline]
    pub fn total_len(&self) -> u8 {
        self.total_len
    }

    /// Copies the content of the ring buffer into the given slice.
    #[inline]
    pub fn copy_to_slice(&self, slice: &mut [T])
    where
        T: Copy,
    {
        self.vec.copy_to_slice(slice);
    }
}

/// Stores the local I/O states of the process.
pub struct IoStates {
    /// The PS/2 keyboard scan-codes that have been received since the process started to be
    /// asleep.
    pub ps2_keyboard: RingBuffer<u8, { PS2Buffer::SIZE }>,
}

impl IoStates {
    /// Creates a new [`IoStates`] instance.
    pub const fn empty() -> Self {
        Self {
            ps2_keyboard: RingBuffer::empty(),
        }
    }
}
