//! Defines the [`Volatile`] type.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// A wrapper around a value which is volatile.
///
/// Reads and writes the the inner value of this type are won't be optimized away by the compiler.
pub struct Volatile<T: ?Sized>(UnsafeCell<T>);

impl<T> Volatile<T> {
    /// Creates a new [`Volatile<T>`] instance from the provided value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }
}

impl<T: ?Sized> Deref for Volatile<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY:
        //  The `Volatile` type respects the regular XOR rules for creating references to the value
        //  it protects. You need a shared reference to the wrapper in order to get a shared
        //  reference to the inner value.
        unsafe { &*self.0.get() }
    }
}

impl<T: ?Sized> DerefMut for Volatile<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY:
        //  The `Volatile` type respects the regular XOR rules for creating references to the value
        //  it protects. You need a mutable reference to the wrapper in order to get a mutable
        //  reference to the inner value.
        unsafe { &mut *self.0.get() }
    }
}

// SAFETY:
//  The `Volatile` type respects the regular XOR rules for creating references to the value it
//  protects. If `T` is `Send`, then it is safe to send a `Volatile<T>`. Same thing for `Sync`.
unsafe impl<T: ?Sized + Send> Send for Volatile<T> {}
unsafe impl<T: ?Sized + Sync> Sync for Volatile<T> {}

impl<T: Clone> Clone for Volatile<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.deref().clone())
    }
}

impl<T: ?Sized> AsRef<T> for Volatile<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized> AsMut<T> for Volatile<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for Volatile<T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.deref(), f)
    }
}
