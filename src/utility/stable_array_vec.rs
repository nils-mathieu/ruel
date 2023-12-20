use core::mem::MaybeUninit;

use crate::global::OutOfMemory;

use super::BumpAllocator;

/// A stable collection of elements.
pub struct StableFixedVec<T: 'static> {
    /// The array of elements.
    array: &'static mut [Slot<T>],

    /// The index of the first free element in the array.
    next_free: usize,
}

impl<T> StableFixedVec<T> {
    /// Creates a new empty [`StableFixedVec`] with the given capacity.
    pub fn new(
        bootstrap_allocator: &mut BumpAllocator,
        capacity: usize,
    ) -> Result<Self, OutOfMemory> {
        let array = bootstrap_allocator.allocate_slice(capacity)?;

        Ok(Self {
            array: crate::utility::init_slice_with(array, |_| Slot::empty()),
            next_free: 0,
        })
    }

    /// Pushes a new value into the vector, returning the index assigned to it.
    pub fn push(&mut self, value: T) -> Result<usize, T> {
        if let Some(slot) = self.array.get_mut(self.next_free) {
            debug_assert!(!slot.is_present());
            slot.write_unchecked(value);
            let index = self.next_free;
            while self.array.get(self.next_free).is_some_and(Slot::is_present) {
                self.next_free += 1;
            }
            Ok(index)
        } else {
            Err(value)
        }
    }

    /// Returns whether the given index is currently occupied by an element.
    #[inline]
    pub fn is_present(&self, index: usize) -> bool {
        self.array.get(index).is_some_and(Slot::is_present)
    }

    /// Returns the value at the given index, without checking without that entry is currently
    /// occupied or not.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        unsafe { self.array.get_unchecked_mut(index).read_unchecked_mut() }
    }

    /// Returns the value at the given index, checking whether that entry is currently occupied or
    /// not.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.array.get_mut(index).and_then(Slot::read_mut)
    }

    /// Returns an iterator over the values of the vector.
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        let maybe_init_slots =
            unsafe { core::slice::from_raw_parts(self.array.as_ptr(), self.next_free) };
        Iter(maybe_init_slots.iter())
    }

    /// Returns an iterator over the values of the vector.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<T> {
        let maybe_init_slots =
            unsafe { core::slice::from_raw_parts_mut(self.array.as_mut_ptr(), self.next_free) };
        IterMut(maybe_init_slots.iter_mut())
    }
}

impl<T> Drop for StableFixedVec<T> {
    fn drop(&mut self) {
        unsafe { core::ptr::drop_in_place::<[Slot<T>]>(self.array) }
    }
}

/// A slot in a [`StableFixedVec`].
struct Slot<T> {
    /// The value stored in the slot.
    val: MaybeUninit<T>,
    /// Whether the slot is currently occupied by an element.
    present: bool,
}

impl<T> Slot<T> {
    /// Creates a new empty slot.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            val: MaybeUninit::uninit(),
            present: false,
        }
    }

    /// Returns whether the slot is currently occupied by an element.
    #[inline]
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// Overwrites the value in the slot without checking whether the slot is currently occupied.
    ///
    /// If the slot is currently occupied, its previous value will leak.
    #[inline]
    pub fn write_unchecked(&mut self, val: T) {
        self.val.write(val);
        self.present = true;
    }

    /// Reads the value in the slot without checking whether the slot is currently occupied.
    ///
    /// # Safety
    ///
    /// If the slot is not currently occupied, the returned value will be uninitialized. This is
    /// instant undefined behavior.
    #[inline]
    pub unsafe fn read_unchecked_mut(&mut self) -> &mut T {
        unsafe { self.val.assume_init_mut() }
    }

    /// Reads the value in the slot without checking whether the slot is currently occupied.
    ///
    /// # Safety
    ///
    /// If the slot is not currently occupied, the returned value will be uninitialized. This is
    /// instant undefined behavior.
    #[inline]
    pub unsafe fn read_unchecked(&self) -> &T {
        unsafe { self.val.assume_init_ref() }
    }

    /// Reads the value in the slot, checking whether the slot is currently occupied.
    #[inline]
    pub fn read(&self) -> Option<&T> {
        if self.present {
            Some(unsafe { self.read_unchecked() })
        } else {
            None
        }
    }

    /// Reads the value in the slot, checking whether the slot is currently occupied.
    #[inline]
    pub fn read_mut(&mut self) -> Option<&mut T> {
        if self.present {
            Some(unsafe { self.read_unchecked_mut() })
        } else {
            None
        }
    }
}

impl<T> Drop for Slot<T> {
    fn drop(&mut self) {
        if self.present {
            unsafe { self.val.assume_init_drop() };
        }
    }
}

impl<'a, T> IntoIterator for &'a StableFixedVec<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut StableFixedVec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over the values of a [`StableFixedVec`].
pub struct Iter<'a, T>(core::slice::Iter<'a, Slot<T>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.find_map(Slot::read)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.len();
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        (&mut self.0).filter_map(Slot::read).next_back()
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.0
            .as_slice()
            .iter()
            .filter(|slot| slot.is_present())
            .count()
    }
}

impl<T> core::iter::FusedIterator for Iter<'_, T> {}

/// An iterator over the values of a [`StableFixedVec`].
pub struct IterMut<'a, T>(core::slice::IterMut<'a, Slot<T>>);

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.find_map(Slot::read_mut)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.len();
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for IterMut<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        (&mut self.0).filter_map(Slot::read_mut).next_back()
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.0
            .as_slice()
            .iter()
            .filter(|slot| slot.is_present())
            .count()
    }
}

impl<T> core::iter::FusedIterator for IterMut<'_, T> {}
