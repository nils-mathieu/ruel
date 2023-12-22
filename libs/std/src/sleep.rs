/// A trait for types that can be transmuted into a [`sys::WakeUp`].
///
/// # Safety
///
/// Implementors of this trait must be able to be transmuted into a [`sys::WakeUp`] instance
/// safely.
pub unsafe trait WakeUp {
    /// A default instance of the type.
    const DEFAULT: Self;
}

/// A [`WakeUp`] implementation that requests the kernel to wake the process up immediately without
/// blocking.
pub struct Now(sys::WakeUpNow);

unsafe impl WakeUp for Now {
    const DEFAULT: Self = Self(sys::WakeUpNow {
        tag: sys::WakeUpTag::NOW,
    });
}

/// A [`WakeUp`] implementation that requests the kernel to wake the process up when the PS/2
/// keyboard has sent some data.
pub struct PS2Keyboard(sys::WakeUpPS2Keyboard);

unsafe impl WakeUp for PS2Keyboard {
    const DEFAULT: Self = Self(sys::WakeUpPS2Keyboard {
        tag: sys::WakeUpTag::PS2_KEYBOARD,
        length: 0,
        scancodes: [0; sys::WakeUpPS2Keyboard::SIZE],
    });
}

impl PS2Keyboard {
    /// The maximum number of bytes that can be received by the program during a single quantum.
    pub const SIZE: usize = sys::WakeUpPS2Keyboard::SIZE;

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
        self.total_length().saturating_sub(Self::SIZE)
    }

    /// Returns whether some bytes have been dropped since the last time the buffer was read.
    #[inline]
    pub fn has_dropped_bytes(&self) -> bool {
        self.total_length() > Self::SIZE
    }

    /// Returns the scan-codes that have been received by the application since the last time
    /// the buffer was read.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        let len = self.total_length().min(Self::SIZE);
        unsafe { core::slice::from_raw_parts(self.0.scancodes.as_ptr(), len) }
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
}

/// A [`WakeUp`] implementation that requests the kernel to wake the process up when the PS/2
/// mouse has sent some data.
pub struct PS2Mouse(sys::WakeUpPS2Mouse);

unsafe impl WakeUp for PS2Mouse {
    const DEFAULT: Self = Self(sys::WakeUpPS2Mouse {
        tag: sys::WakeUpTag::PS2_MOUSE,
        dx: 0,
        dy: 0,
        flags: sys::WakeUpPS2MouseFlags::empty(),
    });
}

impl PS2Mouse {
    /// Returns whether the left mouse button is currently being pressed.
    #[inline]
    pub fn left_pressed(&self) -> bool {
        self.0
            .flags
            .intersects(sys::WakeUpPS2MouseFlags::LEFT_BUTTON)
    }

    /// Returns whether the right mouse button is currently being pressed.
    #[inline]
    pub fn right_pressed(&self) -> bool {
        self.0
            .flags
            .intersects(sys::WakeUpPS2MouseFlags::RIGHT_BUTTON)
    }

    /// Returns whether the middle mouse button is currently being pressed.
    #[inline]
    pub fn middle_pressed(&self) -> bool {
        self.0
            .flags
            .intersects(sys::WakeUpPS2MouseFlags::MIDDLE_BUTTON)
    }

    /// Returns whether the fourth mouse button is currently being pressed.
    #[inline]
    pub fn fourth_pressed(&self) -> bool {
        self.0
            .flags
            .intersects(sys::WakeUpPS2MouseFlags::FOURTH_BUTTON)
    }

    /// Returns whether the fifth mouse button is currently being pressed.
    #[inline]
    pub fn fifth_pressed(&self) -> bool {
        self.0
            .flags
            .intersects(sys::WakeUpPS2MouseFlags::FIFTH_BUTTON)
    }

    /// Returns whether the mouse has moved.
    #[inline]
    pub fn mouse_moved(&self) -> bool {
        self.0.dx != 0 || self.0.dy != 0
    }

    /// Returns the amount of movement of the mouse on the horizontal axis since the last time the
    /// process read the buffer.
    ///
    /// Negative values means that the mouse moved left, and positive values means that the mouse
    /// moved right.
    #[inline]
    pub fn delta_x(&self) -> i8 {
        self.0.dx
    }

    /// Returns the amount of movement of the mouse on the vertical axis since the last time the
    /// process read the buffer.
    ///
    /// Negative values means that the mouse moved up, and positive values means that the mouse
    /// moved down.
    #[inline]
    pub fn delta_y(&self) -> i8 {
        self.0.dy
    }

    /// Returns whether the mouse has moved or any of the buttons have changed since the last time
    /// the process read the buffer.
    #[inline]
    pub fn changed(&self) -> bool {
        self.0.flags.intersects(sys::WakeUpPS2MouseFlags::CHANGED)
    }
}

/// Sleeps until any of the [`WakeUp`] implementations completes.
#[macro_export]
macro_rules! sleep {
    (
        $result:ident ;
        $($name:ident: $type:ty),* $(,)?
    ) => {
        #[repr(C)]
        struct __WakeUpStruct {
            $($name: $type,)*
        }

        let $result;
        let __WakeUpStruct { $($name,)* } = {
            // Make sure that all the types implement `WakeUp`.
            #[allow(non_snake_case)]
            const fn __implements_WakeUp<T: $crate::sleep::WakeUp>() {}
            $(
                const _: () = __implements_WakeUp::<$type>();
            )*

            let mut contents = __WakeUpStruct {
                $($name: <$type as $crate::sleep::WakeUp>::DEFAULT,)*
            };

            let wake_ups = &mut contents as *mut __WakeUpStruct as *mut $crate::sys::WakeUp;
            let len = core::mem::size_of::<__WakeUpStruct>() / core::mem::size_of::<$crate::sys::WakeUp>();

            $result = $crate::sys::sleep(wake_ups, len);

            contents
        };
    };
}
