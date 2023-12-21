use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use crate::utility::RestoreInterrupts;

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
    #[track_caller]
    pub fn lock(&self) -> MutexGuard<T> {
        let without_interrupts = RestoreInterrupts::without_interrupts();

        if self
            .locked
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_ok()
        {
            // Fast path: no spinning required.
            MutexGuard {
                without_interrupts,
                locked: &self.locked,
                value: unsafe { &mut *self.value.get() },
            }
        } else {
            // Slow path: spin until the lock is released.
            self.lock_cold(without_interrupts)
        }
    }

    /// The cold part fo the locking mechanism.
    #[cold]
    #[track_caller]
    fn lock_cold(&self, without_interrupts: Option<RestoreInterrupts>) -> MutexGuard<T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        MutexGuard {
            without_interrupts,
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
    /// The interrupts state before the lock was acquired.
    without_interrupts: Option<RestoreInterrupts>,
}

impl<'a, T> MutexGuard<'a, T> {
    /// Maps the inner value to a new value.
    pub fn map<U>(self, f: impl FnOnce(&mut T) -> &mut U) -> MutexGuard<'a, U> {
        let without_interrupts = unsafe { core::ptr::read(&self.without_interrupts) };
        let value = unsafe { core::ptr::read(&self.value) };

        MutexGuard {
            locked: self.locked,
            without_interrupts,
            value: f(value),
        }
    }

    /// Maps the inner value to a new value, returning an error if the closure returns an error.
    pub fn try_map<U, E>(
        self,
        f: impl FnOnce(&mut T) -> Result<&mut U, E>,
    ) -> Result<MutexGuard<'a, U>, E> {
        let without_interrupts = unsafe { core::ptr::read(&self.without_interrupts) };
        let value = unsafe { core::ptr::read(&self.value) };

        Ok(MutexGuard {
            locked: self.locked,
            without_interrupts,
            value: f(value)?,
        })
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
