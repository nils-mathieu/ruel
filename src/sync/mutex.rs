use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

/// A mutual exclusion primitive based on spin-locks.
pub struct Mutex<T: ?Sized> {
    /// Whether the mutex is currently locked or not.
    locked: AtomicBool,
    /// The protected value.
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new [`Mutex<T>`] instance.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Locks the mutex and returns a guard that releases the lock when dropped.
    #[inline]
    pub fn lock(&self) -> MutexGuard<T> {
        if self
            .locked
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_ok()
        {
            // Fast path: no spinning required.
            MutexGuard {
                locked: &self.locked,
                value: unsafe { &mut *self.value.get() },
            }
        } else {
            // Slow path: spin until the lock is released.
            self.lock_cold()
        }
    }

    /// The cold part fo the locking mechanism.
    #[cold]
    fn lock_cold(&self) -> MutexGuard<T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        MutexGuard {
            locked: &self.locked,
            value: unsafe { &mut *self.value.get() },
        }
    }
}

/// A guard that releases a lock when dropped.
pub struct MutexGuard<'a, T: ?Sized> {
    /// The lock that we are responsible for releasing.
    locked: &'a AtomicBool,
    /// The protected value.
    value: &'a mut T,
}

impl<'a, T> MutexGuard<'a, T> {
    /// Leaks the protected value, leaving the mutex forever locked.
    #[inline]
    pub fn leak(self) -> &'a mut T {
        let value = unsafe { core::ptr::read(&self.value) };
        core::mem::forget(self);
        value
    }

    /// Maps the inner value to a new value.
    pub fn map<U>(self, f: impl FnOnce(&mut T) -> &mut U) -> MutexGuard<'a, U> {
        MutexGuard {
            locked: self.locked,
            value: f(self.leak()),
        }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.locked.store(false, Release);
    }
}
