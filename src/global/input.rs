use crate::utility::array_vec::ArrayVec;

/// The maximum number of scancodes that the PS/2 keyboard can sent during a single quantum.
const PS2_KEYBOARD_BUFFER_SIZE: usize = 16;

/// Contains a bunch of buffers used to store inputs from the user in case some process
/// needs them.
pub struct Inputs {
    /// The buffer used to store scancodes from the PS/2 keyboard.
    pub ps2_keyboard: ArrayVec<u8, PS2_KEYBOARD_BUFFER_SIZE>,
}

impl Inputs {
    /// Creates a new empty [`Inputs`] collection.
    pub const fn empty() -> Self {
        Self {
            ps2_keyboard: ArrayVec::new_array(),
        }
    }

    /// Clears the buffers of this [`Inputs`] collection.
    pub fn clear(&mut self) {
        self.ps2_keyboard.clear();
    }
}
