use core::cell::UnsafeCell;
use core::ffi::{c_char, CStr};

/// A pointer into the bootloader-reclaimable memory of the kernel.
#[repr(transparent)]
pub struct LiminePtr<T: ?Sized>(*const T);

impl<T> LiminePtr<T> {
    /// A null [`LiminePtr<T>`].
    pub const NULL: Self = Self(core::ptr::null());

    /// Creates a slice from this pointer.
    ///
    /// # Safety
    ///
    /// The memory pointed to by this pointer must be valid the lifetime of the created slice.
    #[inline]
    pub unsafe fn slice<'a>(self, len: usize) -> &'a [T] {
        unsafe { core::slice::from_raw_parts(self.0, len) }
    }
}

impl<T: ?Sized> LiminePtr<T> {
    /// Returns the inner raw pointer.
    #[inline]
    pub const fn as_ptr(self) -> *const T {
        self.0
    }

    /// Returns whether the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Turns this pointer into a regular Rust reference.
    ///
    /// # Safety
    ///
    /// The memory pointed to by this pointer must be valid.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> &'a T {
        unsafe { &*self.0 }
    }

    /// Casts this pointer to a pointer to a different type.
    #[inline]
    pub const fn cast<U>(self) -> LiminePtr<U> {
        LiminePtr(self.0 as *const U)
    }
}

impl LiminePtr<c_char> {
    /// Turns this pointer into a C string.
    ///
    /// # Safety
    ///
    /// The memory pointed to by this pointer must be valid and null-terminated.
    #[inline]
    pub unsafe fn as_cstr<'a>(self) -> &'a CStr {
        unsafe { CStr::from_ptr(self.0) }
    }
}

impl<T: ?Sized> core::fmt::Debug for LiminePtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: ?Sized> Clone for LiminePtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for LiminePtr<T> {}

// SAFETY:
//  `LiminePtr` is not necessarily a unique poiner to the value it points to (though in the
//  context of the Limine boot protocol, that may be the case). If `T` is `Sync`, then it is safe
//  to send a `ResponsePtr<T>` to another thread.
unsafe impl<T: ?Sized + Sync> Send for LiminePtr<T> {}
unsafe impl<T: ?Sized + Sync> Sync for LiminePtr<T> {}

/// A pointer to a response provided by the bootloader.
///
/// # Volatile Semantics
///
/// Reads the inner value are volatile, so that the compiler doesn't attempt to optimize those
/// away. This is require because the bootloader will write to this value at *load time*, meaning
/// that the compiler has no idea that the value will change and will assume (as it should) that
/// the value never actually changes.
#[derive(Debug)]
pub struct ResponsePtr<T: ?Sized>(UnsafeCell<LiminePtr<T>>);

impl<T> ResponsePtr<T> {
    /// A null [`ResponsePtr<T>`].
    #[allow(clippy::declare_interior_mutable_const)]
    pub const NULL: Self = Self(UnsafeCell::new(LiminePtr::NULL));
}

impl<T: ?Sized> ResponsePtr<T> {
    /// Returns the (eventually null) raw pointer.
    #[inline]
    pub fn read_raw(&self) -> LiminePtr<T> {
        // We need to use volatile semantics because the compiler may optimize away the reads of
        // the pointer.
        //
        // This is beacuse from the point of view of the compiler, the pointer is never written to
        // (and thus always null).
        //
        // Note that in practice, using a function like `read_volatile` would not work in this case
        // because the function takes `self` by value, meaning that the compiler has already
        // technically copied the "null" pointer here. Even if we read-volatile it, it's already
        // null.
        //
        // Instead, we use an `UnsafeCell`, because that type specifically prevents Rust from
        // optimizing away reads of the inner value (because that value could have been written
        // by another thread).

        unsafe { *self.0.get() }
    }

    /// Returns a reference to the pointed value, if the pointer is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is actually valid.
    ///
    /// # Remarks
    ///
    /// If the [`ResponsePtr`] is only written to by the bootloader (and that the bootloader
    /// is actually compliant with the standard), then this function is safe to call as long
    /// as the bootloader-reclaimable memory region is not overwritten.
    #[inline]
    pub unsafe fn read<'a>(&self) -> Option<&'a T> {
        let p = self.read_raw();

        if p.is_null() {
            None
        } else {
            Some(unsafe { p.as_ref() })
        }
    }
}

impl<T: ?Sized> Clone for ResponsePtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(UnsafeCell::new(self.read_raw()))
    }
}

unsafe impl<T: ?Sized + Sync> Send for ResponsePtr<T> {}
unsafe impl<T: ?Sized + Sync> Sync for ResponsePtr<T> {}
