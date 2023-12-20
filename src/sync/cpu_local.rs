use core::ops::Deref;

use crate::global::OutOfMemory;
use crate::utility::BumpAllocator;

/// A value that's duplicated for each CPU.
pub struct CpuLocal<T: 'static> {
    values: &'static mut [T],
}

impl<T> CpuLocal<T> {
    /// Creates a new [`CpuLocal<T>`] instance with the correct number of entries.
    ///
    /// This function will use the provided `new` function to create the initial value for each
    /// CPU.
    pub fn new_with(
        bootstrap_allocator: &mut BumpAllocator,
        mut new: impl FnMut() -> T,
    ) -> Result<Self, OutOfMemory> {
        let num_cpus = 1;

        let values = bootstrap_allocator.allocate_slice::<T>(num_cpus)?;

        Ok(Self {
            values: crate::utility::init_slice_with(values, |_| new()),
        })
    }

    /// Creates a new [`CpuLocal<T>`] instance with the correct number of entries.
    ///
    /// This function will use the provided `Default` implementation to create the initial value
    /// for each CPU.
    pub fn new(bootstrap_allocator: &mut BumpAllocator) -> Result<Self, OutOfMemory>
    where
        T: Default,
    {
        Self::new_with(bootstrap_allocator, T::default)
    }
}

// SAFETY:
//  The `CpuLocal` type only allows access each individual CPU a single value.
unsafe impl<T> Sync for CpuLocal<T> {}
unsafe impl<T> Send for CpuLocal<T> {}

impl<T> Deref for CpuLocal<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // TODO: Actually use A CPU ID when we have multiple CPUs running.
        assert!(self.values.len() == 1);

        // SAFETY:
        //  We made sure that only one CPU is running.
        unsafe { self.values.get_unchecked(0) }
    }
}

impl<T> Drop for CpuLocal<T> {
    fn drop(&mut self) {
        unsafe { core::ptr::drop_in_place(self.values) }
    }
}
