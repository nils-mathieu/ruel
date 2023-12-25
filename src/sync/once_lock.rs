//! Defines the [`OnceLock`] type.

use core::cell::UnsafeCell;
use core::convert::Infallible;
use core::mem::MaybeUninit;
use core::sync::atomic::Ordering::{Acquire, Release};

use crate::utility::RestoreInterrupts;

use self::state::*;

/// A lock which can only be initialized once and then accessed immutably.
pub struct OnceLock<T> {
    /// The current state of the [`OnceLock<T>`].
    state: AtomicState,
    /// The inner value that is in the process of being initialized.
    value: UnsafeCell<MaybeUninit<T>>,
}

// This types allws initializing the value on a different thread than the one that will drop it,
// so `T` must be `Send` in order for the `OnceLock<T>` to be `Sync`.
unsafe impl<T: Send> Send for OnceLock<T> {}
unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}

impl<T> OnceLock<T> {
    /// Creates a new uninitialized [`OnceLock<T>`] instance.
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: AtomicState::new(State::Uninitialized),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Returns the inner value protected by this [`OnceLock<T>`] instance.
    ///
    /// # Safety
    ///
    /// This function may only be called once the [`OnceLock<T>`] instance has been initialized.
    #[inline]
    pub unsafe fn get_unchecked(&self) -> &T {
        unsafe { (*self.value.get()).assume_init_ref() }
    }

    /// Returns whether the [`OnceLock<T>`] instance has been initialized.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.state.load(Acquire) == State::Initialized
    }

    /// Attempts to get the inner value of this [`OnceLock<T>`] instance.
    ///
    /// # Returns
    ///
    /// If the value is not initialized yet, this function returns [`None`]. Otherwise, the value
    /// is returned.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.is_initialized() {
            // SAFETY:
            //  Because we just used Acquire ordering, we know for sure that the value is
            //  actually initialized.
            Some(unsafe { self.get_unchecked() })
        } else {
            None
        }
    }

    /// Returns a reference to the inner value of this [`OnceLock<T>`] instance, initializing it
    /// with the provided function if it is not initialized yet.
    pub fn get_or_init(&self, init: impl FnOnce() -> T) -> &T {
        match self.get_or_try_init(|| Ok::<T, Infallible>(init())) {
            Ok(value) => value,
            Err(err) => match err {},
        }
    }

    /// Attempts to get the inner value of this [`OnceLock<T>`] instance, or initializes it with
    /// the provided function if it is not initialized yet.
    pub fn get_or_try_init<E>(&self, init: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        if let Some(value) = self.get() {
            // Fast path: the value is already initialized.
            Ok(value)
        } else {
            // Slow path: we need to initialize the value.
            // Using a cold function hints the compiler that this path is unlikely to be taken.
            self.get_or_try_init_cold(init)
        }
    }

    #[cold]
    fn get_or_try_init_cold<E>(&self, init: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        let _restore_interrupts = RestoreInterrupts::without_interrupts();

        loop {
            let state =
                self.state.compare_exchange_weak(
                    State::Uninitialized,
                    State::Initializing,
                    Acquire,
                    Acquire,
                );

            match state {
                // We successfully transitioned from `Uninitialized` to `Initializing`, meaning
                // that we are responsible for calling the `init` function to initialize the
                // value.
                Ok(_) => break,

                // The compare-exchange failed. Let's try to perform the transition again.
                Err(State::Uninitialized) => (),

                // Some other thread has initialized the value. We can just return it.
                Err(State::Initialized) => return Ok(unsafe { self.get_unchecked() }),

                // Some other thread started initializing the value. We need to wait until it is
                // done.
                Err(State::Initializing) => match self.poll() {
                    Some(value) => return Ok(value),

                    // The thread failed to initialize the value. We need to try again.
                    None => continue,
                },
            }
        }

        // We are responsible for initializing the value.
        // This guard will ensure that the state of the lock is properly updated once the
        // initialization routine is done. By default, the `out_state` is set to `Uninitialized`
        // in order to handle panics and errors in the initialization routine. If the function
        // successfully returns, however, the `out_state` is set to `Initialized`.

        struct Guard<'a> {
            state: &'a AtomicState,
            out_state: State,
        }

        impl<'a> Drop for Guard<'a> {
            fn drop(&mut self) {
                self.state.store(self.out_state, Release);
            }
        }

        let mut guard = Guard {
            state: &self.state,
            out_state: State::Uninitialized,
        };

        let val = init()?;

        unsafe {
            // SAFETY:
            //  We acquired the lock (with the `Initializing` state), we are therefor the only
            //  thread accessing the value.
            (*self.value.get()).write(val);
        }

        guard.out_state = State::Initialized;

        return Ok(unsafe { self.get_unchecked() });
    }

    /// If the [`OnceLock<T>`] is currently in the process of being initialized, this function
    /// waits until the initialization routine is done and returns the initialized value.
    pub fn poll(&self) -> Option<&T> {
        loop {
            match self.state.load(Acquire) {
                State::Uninitialized => return None,
                State::Initializing => core::hint::spin_loop(),
                State::Initialized => return Some(unsafe { self.get_unchecked() }),
            }
        }
    }
}

mod state {
    use core::mem::transmute;
    use core::sync::atomic::{AtomicU8, Ordering};

    /// A state that a [`OnceLock<T>`] can be in.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum State {
        /// The value is not initialized yet, and no thread has attempted to initialize it.
        Uninitialized,
        /// The value is not initialized yet, but another thread is attempting to initialize it
        /// concurrently.
        Initializing,
        /// The value is initialized.
        Initialized,
    }

    /// An atomic counterpart to [`State`].
    #[repr(transparent)]
    pub struct AtomicState(AtomicU8);

    impl AtomicState {
        /// Creates a new [`AtomicState`] instance.
        #[inline]
        pub const fn new(state: State) -> Self {
            Self(AtomicU8::new(state as u8))
        }

        /// See [`AtomicU8::load`].
        #[inline]
        pub fn load(&self, ordering: Ordering) -> State {
            // SAFETY:
            //  The inner value of an `AtomicState` is always a valid `State` variant.
            unsafe { transmute(self.0.load(ordering)) }
        }

        /// See [`AtomicU8::compare_exchange_weak`].
        pub fn compare_exchange_weak(
            &self,
            current: State,
            new: State,
            success: Ordering,
            failure: Ordering,
        ) -> Result<State, State> {
            match self
                .0
                .compare_exchange_weak(current as u8, new as u8, success, failure)
            {
                Ok(old) => Ok(unsafe { transmute(old) }),
                Err(old) => Err(unsafe { transmute(old) }),
            }
        }

        /// See [`AtomicU8::store`].
        #[inline]
        pub fn store(&self, state: State, ordering: Ordering) {
            self.0.store(state as u8, ordering)
        }
    }
}
