use core::marker::PhantomData;
use core::mem::{transmute, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::Result;

/// Whether the framebuffers have been acquired by the current process.
static ACQUIRED: AtomicBool = AtomicBool::new(false);

/// Attempts to acquire the framebuffers available to the system.
///
/// # Panics
///
/// This function panics if the current process has already acquired the framebuffers.
pub fn acquire(fbs: &mut [MaybeUninit<Framebuffer>]) -> Result<&mut [Framebuffer]> {
    let mut count = fbs.len();
    match sys::acquire_framebuffers(fbs.as_mut_ptr() as *mut sys::Framebuffer, &mut count) {
        sys::SysResult::SUCCESS => {
            // Acquire the framebuffer lock.
            assert!(
                !ACQUIRED.swap(true, Ordering::Acquire),
                "the framebuffers have already been acquired by the current process"
            );

            Ok(unsafe {
                core::slice::from_raw_parts_mut(fbs.as_mut_ptr() as *mut Framebuffer, count)
            })
        }
        err => Err(err),
    }
}

/// A framebuffer object.
#[derive(Debug)]
pub struct Framebuffer(sys::Framebuffer);

impl Framebuffer {
    /// Returns a slice over the bytes making up the framebuffer.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.0.address, self.0.size()) }
    }

    /// Returns a mutable slice over the bytes making up the framebuffer.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.0.address, self.0.size()) }
    }

    /// Returns the width of the framebuffer.
    #[inline]
    pub fn width(&self) -> usize {
        self.0.width as usize
    }

    /// Returns the height of the framebuffer.
    #[inline]
    pub fn height(&self) -> usize {
        self.0.height as usize
    }

    /// Returns the format of the framebuffer.
    #[inline]
    pub fn format(&self) -> sys::FramebufferFormat {
        self.0.format
    }

    /// Returns the size of the framebuffer, in bytes.
    #[inline]
    pub fn size_in_bytes(&self) -> usize {
        self.0.size()
    }

    /// Returns the number of bytes per pixel of the framebuffer,
    #[inline]
    pub fn bytes_per_pixels(&self) -> usize {
        self.format().bytes_per_pixel() as usize
    }

    /// Returns the number of bytes per line of the framebuffer.
    #[inline]
    pub fn bytes_per_line(&self) -> usize {
        self.0.bytes_per_lines
    }

    /// Converts coordinates to an index in the framebuffer.
    ///
    /// # Remarks
    ///
    /// This function does not check whether either X or Y are in bounds or not.
    #[inline]
    fn index_of(&self, x: usize, y: usize) -> usize {
        let bpl = self.0.bytes_per_lines;
        let bpp = self.bytes_per_pixels();

        y * bpl + x * bpp
    }

    /// Returns the address of the pixel at the given coordinates.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - `x` is less than the width of the framebuffer.
    ///
    /// - `y` is less than the height of the framebuffer.
    #[inline]
    pub unsafe fn pixel_address(&self, x: usize, y: usize) -> *mut u8 {
        unsafe { self.0.address.add(self.index_of(x, y)) }
    }

    /// Creates a new [`TypedFramebuffer<T>`] view of this framebuffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `T` is the correct type for the framebuffer.
    #[inline]
    pub unsafe fn into_typed_unchecked<T: FramebufferFormat>(self) -> TypedFramebuffer<T> {
        TypedFramebuffer {
            inner: self,
            format: PhantomData,
        }
    }

    /// Creates a new [`TypedFramebuffer<T>`] view of this framebuffer without checking whether
    /// `T` is the correct type for the framebuffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `T` is the correct type for the framebuffer.
    #[inline]
    pub unsafe fn typed_unchecked_ref<T>(&self) -> &TypedFramebuffer<T> {
        unsafe { transmute::<&Self, &TypedFramebuffer<T>>(self) }
    }

    /// Creates a new [`TypedFramebuffer<T>`] view of this framebuffer without checking whether
    /// `T` is the correct type for the framebuffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `T` is the correct type for the framebuffer.
    #[inline]
    pub unsafe fn typed_unchecked_mut<T>(&mut self) -> &mut TypedFramebuffer<T> {
        unsafe { transmute::<&mut Self, &mut TypedFramebuffer<T>>(self) }
    }

    /// Creates a new [`TypedFramebuffer<T>`] view of this framebuffer.
    ///
    /// # Errors
    ///
    /// This function returns [`None`] if `T` is not the correct type for the framebuffer.
    #[inline]
    pub fn into_typed<T: FramebufferFormat>(self) -> Option<TypedFramebuffer<T>> {
        if self.format() == T::FORMAT {
            Some(unsafe { self.into_typed_unchecked() })
        } else {
            None
        }
    }

    /// Creates a new [`TypedFramebuffer<T>`] view of this framebuffer.
    ///
    /// # Errors
    ///
    /// This function returns [`None`] if `T` is not the correct type for the framebuffer.
    #[inline]
    pub fn typed_ref<T: FramebufferFormat>(&self) -> Option<&TypedFramebuffer<T>> {
        if self.format() == T::FORMAT {
            Some(unsafe { self.typed_unchecked_ref() })
        } else {
            None
        }
    }

    /// Creates a new [`TypedFramebuffer<T>`] view of this framebuffer.
    ///
    /// # Errors
    ///
    /// This function returns [`None`] if `T` is not the correct type for the framebuffer.
    #[inline]
    pub fn typed_mut<T: FramebufferFormat>(&mut self) -> Option<&mut TypedFramebuffer<T>> {
        if self.format() == T::FORMAT {
            Some(unsafe { self.typed_unchecked_mut() })
        } else {
            None
        }
    }
}

/// A framebuffer with an associated [`FramebufferFormat`].
#[derive(Debug)]
#[repr(transparent)]
pub struct TypedFramebuffer<T> {
    inner: Framebuffer,
    format: PhantomData<T>,
}

impl<T: FramebufferFormat> TypedFramebuffer<T> {
    /// Reads a pixel from the framebuffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `x` is less than the width of the framebuffer, and that
    /// `y` is less than the height of the framebuffer.
    #[inline]
    pub unsafe fn pixel_unchecked(&self, x: usize, y: usize) -> T {
        unsafe { T::decode(self.pixel_address(x, y)) }
    }

    /// Reads a pixel from the framebuffer.
    ///
    /// # Panics
    ///
    /// This function panics if `x` or `y` is outside the bounds of the framebuffer.
    #[inline]
    pub fn pixel(&self, x: usize, y: usize) -> T {
        assert!(x < self.width());
        assert!(y < self.height());
        unsafe { self.pixel_unchecked(x, y) }
    }

    /// Writes a pixel to the framebuffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `x` is less than the width of the framebuffer, and that
    /// `y` is less than the height of the framebuffer.
    #[inline]
    pub unsafe fn set_pixel_unchecked(&mut self, x: usize, y: usize, pixel: T) {
        unsafe { pixel.encode(self.pixel_address(x, y)) }
    }

    /// Writes a pixel to the framebuffer.
    ///
    /// # Panics
    ///
    /// This function panics if `x` or `y` is outside the bounds of the framebuffer.
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: T) {
        assert!(x < self.width(), "x = {}, width = {}", x, self.width());
        assert!(y < self.height(), "y = {}, height = {}", y, self.height());
        unsafe { self.set_pixel_unchecked(x, y, pixel) };
    }
}

impl<T> Deref for TypedFramebuffer<T> {
    type Target = Framebuffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for TypedFramebuffer<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A color in a framebuffer, encoded as 32-bit RGB.
///
/// Corresponds to [`sys::FramebufferFormat::BGR32`].
#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Bgr32(pub Rgb);

impl FramebufferFormat for Bgr32 {
    const FORMAT: sys::FramebufferFormat = sys::FramebufferFormat::BGR32;

    #[inline]
    unsafe fn decode(src: *const u8) -> Self {
        #[cfg(target_endian = "little")]
        unsafe {
            Self(Rgb {
                red: *src.add(2),
                green: *src.add(1),
                blue: *src,
            })
        }

        #[cfg(target_endian = "big")]
        unsafe {
            Self(Rgb {
                red: *src,
                green: *src.add(1),
                blue: *src.add(2),
            })
        }
    }

    #[inline]
    unsafe fn encode(self, dst: *mut u8) {
        #[cfg(target_endian = "little")]
        unsafe {
            *dst = self.0.blue;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.red;
        }

        #[cfg(target_endian = "big")]
        unsafe {
            *dst = self.0.red;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.blue;
        }
    }
}

/// A color in a framebuffer, encoded as 32-bit RGB.
///
/// Corresponds to [`sys::FramebufferFormat::RGB32`].
pub struct Rgb32(pub Rgb);

impl FramebufferFormat for Rgb32 {
    const FORMAT: sys::FramebufferFormat = sys::FramebufferFormat::RGB32;

    #[inline]
    unsafe fn decode(src: *const u8) -> Self {
        #[cfg(target_endian = "little")]
        unsafe {
            Self(Rgb {
                red: *src.add(2),
                green: *src.add(1),
                blue: *src,
            })
        }

        #[cfg(target_endian = "big")]
        unsafe {
            Self(Rgb {
                red: *src,
                green: *src.add(1),
                blue: *src.add(2),
            })
        }
    }

    #[inline]
    unsafe fn encode(self, dst: *mut u8) {
        #[cfg(target_endian = "little")]
        unsafe {
            *dst = self.0.blue;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.red;
        }

        #[cfg(target_endian = "big")]
        unsafe {
            *dst = self.0.red;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.blue;
        }
    }
}

/// A color in a framebuffer, encoded as 24-bit RGB.
///
/// Corresponds to [`sys::FramebufferFormat::BGR24`].
pub struct Bgr24(pub Rgb);

impl FramebufferFormat for Bgr24 {
    const FORMAT: sys::FramebufferFormat = sys::FramebufferFormat::BGR24;

    #[inline]
    unsafe fn decode(src: *const u8) -> Self {
        #[cfg(target_endian = "little")]
        unsafe {
            Self(Rgb {
                red: *src.add(2),
                green: *src.add(1),
                blue: *src,
            })
        }

        #[cfg(target_endian = "big")]
        unsafe {
            Self(Rgb {
                red: *src,
                green: *src.add(1),
                blue: *src.add(2),
            })
        }
    }

    #[inline]
    unsafe fn encode(self, dst: *mut u8) {
        #[cfg(target_endian = "little")]
        unsafe {
            *dst = self.0.blue;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.red;
        }

        #[cfg(target_endian = "big")]
        unsafe {
            *dst = self.0.red;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.blue;
        }
    }
}

/// A color in a framebuffer, encoded as 24-bit RGB.
///
/// Corresponds to [`sys::FramebufferFormat::RGB24`].
pub struct Rgb24(pub Rgb);

impl FramebufferFormat for Rgb24 {
    const FORMAT: sys::FramebufferFormat = sys::FramebufferFormat::RGB24;

    #[inline]
    unsafe fn decode(src: *const u8) -> Self {
        #[cfg(target_endian = "little")]
        unsafe {
            Self(Rgb {
                red: *src.add(2),
                green: *src.add(1),
                blue: *src,
            })
        }

        #[cfg(target_endian = "big")]
        unsafe {
            Self(Rgb {
                red: *src,
                green: *src.add(1),
                blue: *src.add(2),
            })
        }
    }

    #[inline]
    unsafe fn encode(self, dst: *mut u8) {
        #[cfg(target_endian = "little")]
        unsafe {
            *dst = self.0.blue;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.red;
        }

        #[cfg(target_endian = "big")]
        unsafe {
            *dst = self.0.red;
            *dst.add(1) = self.0.green;
            *dst.add(2) = self.0.blue;
        }
    }
}

/// A way to encode and decode pixels in a framebuffer.
pub trait FramebufferFormat {
    /// The canonical [`sys::FramebufferFormat`] value for this format.
    const FORMAT: sys::FramebufferFormat;

    /// Decodes a pixel from the framebuffer format.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `dst` points to a pixel that's properly encoded in the
    /// format represented by `Self`.
    unsafe fn decode(src: *const u8) -> Self;

    /// Encodes a pixel into the framebuffer format.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `dst` is properly encoded in the format represented by `Self`.
    unsafe fn encode(self, dst: *mut u8);
}

/// A generic RGB color type.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Rgb {
    /// The red component of the color.
    pub red: u8,
    /// The green component of the color.
    pub green: u8,
    /// The blue component of the color.
    pub blue: u8,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0);
    pub const CYAN: Self = Self::new(0, 255, 255);
    pub const MAGENTA: Self = Self::new(255, 0, 255);

    /// Creates a new [`Rgb`] color.
    #[inline]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}
