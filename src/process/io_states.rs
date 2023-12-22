use ruel_sys::{WakeUpPS2Keyboard, WakeUpPS2MouseFlags};

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
    /// The PS/2 keyboard scan-codes that have been received since the last time
    /// the process read the buffer.
    pub ps2_keyboard: RingBuffer<u8, { WakeUpPS2Keyboard::SIZE }>,
    /// The last state reported by the mouse.
    pub ps2_mouse_state: WakeUpPS2MouseFlags,
    /// The offset of the PS/2 mouse since the last time the process read the buffer.
    pub ps2_mouse_offset: [i8; 2],
}

impl IoStates {
    /// Creates a new [`IoStates`] instance.
    pub const fn empty() -> Self {
        Self {
            ps2_keyboard: RingBuffer::empty(),
            ps2_mouse_state: WakeUpPS2MouseFlags::empty(),
            ps2_mouse_offset: [0; 2],
        }
    }

    pub fn clear(&mut self) {
        self.ps2_keyboard.clear();
        self.ps2_mouse_state = WakeUpPS2MouseFlags::empty();
        self.ps2_mouse_offset = [0; 2];
    }
}
