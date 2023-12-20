use core::mem::MaybeUninit;

use super::UninitArray;

/// A vector of fixed size stored inline.
pub type ArrayVecDequeue<T, const N: usize> = FixedVecDequeue<[MaybeUninit<T>; N]>;

/// A vector of fixed size. It cannot grow.
///
/// This type allows push and pop operations to both be O(1) by keeping track of the start and
/// end of the vector.
pub struct FixedVecDequeue<A: ?Sized + UninitArray> {
    start: u32,
    len: u32,
    array: A,
}

impl<A: UninitArray> FixedVecDequeue<A> {
    /// Creates a new [`FixedVecDequeue`] with the given array.
    #[inline]
    pub const fn new(array: A) -> Self {
        Self {
            start: 0,
            len: 0,
            array,
        }
    }
}

impl<T, const N: usize> FixedVecDequeue<[MaybeUninit<T>; N]> {
    /// Creates a new [`FixedVecDequeue`] with an fixed-size array.
    #[inline]
    pub const fn new_array() -> Self {
        Self::new(unsafe { MaybeUninit::uninit().assume_init() })
    }
}

impl<A: ?Sized + UninitArray> FixedVecDequeue<A> {
    /// Returns the length of the vector.
    pub fn as_mut_slices(&mut self) -> (&mut [A::Item], &mut [A::Item]) {
        unsafe {
            if self.start + self.len <= self.capacity() as u32 {
                let p = self.array.as_mut_ptr().add(self.start as usize);
                let len = self.len as usize;
                let slice = core::slice::from_raw_parts_mut(p, len);
                (slice, &mut [])
            } else {
                let p1 = self.array.as_mut_ptr();
                let len1 = (self.start + self.len) as usize % self.capacity();

                let p2 = self.array.as_mut_ptr().add(self.start as usize);
                let len2 = self.array.len() - self.start as usize;

                (
                    core::slice::from_raw_parts_mut(p1, len1),
                    core::slice::from_raw_parts_mut(p2, len2),
                )
            }
        }
    }

    /// Returns the length of the vector.
    pub fn as_slices(&self) -> (&[A::Item], &[A::Item]) {
        unsafe {
            if self.start + self.len <= self.capacity() as u32 {
                let p = self.array.as_ptr().add(self.start as usize);
                let len = self.len as usize;
                let slice = core::slice::from_raw_parts(p, len);
                (slice, &[])
            } else {
                let p1 = self.array.as_ptr();
                let len1 = (self.start + self.len) as usize % self.capacity();

                let p2 = self.array.as_ptr().add(self.start as usize);
                let len2 = self.array.len() - self.start as usize;

                (
                    core::slice::from_raw_parts(p1, len1),
                    core::slice::from_raw_parts(p2, len2),
                )
            }
        }
    }

    /// Copies the content of the ring buffer into the given slice.
    #[inline]
    pub fn copy_to_slice(&self, slice: &mut [A::Item])
    where
        A::Item: Copy,
    {
        assert!(slice.len() >= self.len());

        let (a, b) = self.as_slices();

        unsafe {
            core::ptr::copy_nonoverlapping(a.as_ptr(), slice.as_mut_ptr(), a.len());
            core::ptr::copy_nonoverlapping(b.as_ptr(), slice.as_mut_ptr().add(a.len()), b.len());
        }
    }

    /// Returns the length of the vector.
    pub fn clear(&mut self) {
        unsafe {
            let (a, b) = self.as_mut_slices();
            core::ptr::drop_in_place(a);
            core::ptr::drop_in_place(b);
        }

        self.start = 0;
        self.len = 0;
    }

    /// Returns the length of the vector.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.array.len()
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns the remaining capacity of the vector.
    #[inline]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity() - self.len()
    }

    /// Returns whether the vector is full and cannot store any more elements.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.remaining_capacity() == 0
    }

    // /// Attempts to push a new item into the vector.
    // ///
    // /// If the item cannot be pushed, it is returned back to the caller.
    // #[inline]
    // pub fn try_push(&mut self, value: A::Item) -> Result<(), A::Item> {
    //     if self.is_full() {
    //         return Err(value);
    //     }

    //     unsafe {
    //         let index = (self.start + self.len) as usize % self.capacity();
    //         self.array.as_mut_ptr().add(index).write(value);
    //     }

    //     self.len += 1;

    //     Ok(())
    // }

    /// Pushes a new item into the vector.
    ///
    /// If the vector is full, the first item is overwritten.
    ///
    /// # Returns
    ///
    /// This function returns the eventually replaced value.
    pub fn push_overwrite(&mut self, value: A::Item) -> Option<A::Item> {
        if self.is_full() {
            let ret = unsafe {
                self.array
                    .as_mut_ptr()
                    .add(self.start as usize)
                    .replace(value)
            };

            self.start += 1;
            if self.start == self.capacity() as u32 {
                self.start = 0;
            }

            Some(ret)
        } else {
            unsafe {
                let index = (self.start + self.len) as usize;
                self.array.as_mut_ptr().add(index).write(value);
            }

            self.len += 1;

            None
        }
    }
}

impl<A: ?Sized + UninitArray> Drop for FixedVecDequeue<A> {
    fn drop(&mut self) {
        self.clear();
    }
}
