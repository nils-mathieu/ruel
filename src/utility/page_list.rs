use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr::NonNull;

use crate::cpu::paging::HHDM_OFFSET;
use crate::global::{GlobalToken, OutOfMemory};

/// A linked-list of elements stored in chunks, each of which is a page in size.
pub struct PageList<T> {
    first: Option<NonNull<Node<T>>>,
    glob: GlobalToken,
}

unsafe impl<T: Send> Send for PageList<T> {}
unsafe impl<T: Sync> Sync for PageList<T> {}

impl<T> PageList<T> {
    /// Creates a new empty [`PageList<T>`].
    #[inline]
    pub const fn new(glob: GlobalToken) -> Self {
        Self { first: None, glob }
    }

    /// Creates a new cursor over the list.
    #[inline]
    pub fn cursor(&mut self) -> Option<Cursor<T>> {
        Some(Cursor {
            node: self.first.as_mut()?,
            glob: self.glob,
        })
    }

    /// Pushes a new element anywhere into the list.
    pub fn push_anywhere(&mut self, value: T) -> Result<(), OutOfMemory> {
        let glob = self.glob;

        if let Some(mut cursor) = self.cursor() {
            if cursor.try_find(|slice| slice.len() < Node::<T>::capacity()) {
                // Found a suitable node.

                unsafe {
                    cursor.as_mut_ptr().add(cursor.len()).write(value);
                    cursor.set_len(cursor.len() + 1);
                }
            } else {
                // No suitable node found.

                let node = Node::new(glob)?;

                unsafe {
                    debug_assert!(cursor.node.as_ref().next.is_none());
                    cursor.node.as_mut().next = Some(node);
                }
            }
        } else {
            // The list is empty.

            let node = Node::new(self.glob)?;
            self.first = Some(node);
        }

        Ok(())
    }
}

/// A node in a [`PageList<T>`].
///
/// This node is always aligned to the size of a page.
#[repr(C, align(4096))]
struct Node<T> {
    /// The pointer to the next node in the list.
    next: Option<NonNull<Node<T>>>,
    /// The length of the node.
    ///
    /// The most significant bit indicates that the `next` field is initialized.
    len: usize,
    /// The type stored in the node.
    _marker: core::marker::PhantomData<T>,
}

impl<T> Node<T> {
    /// The maximum number of elements that can be stored in a single node.
    #[inline]
    pub const fn capacity() -> usize {
        (4096 - size_of::<Self>()) / size_of::<T>()
    }

    /// Allocates a new node with one element.
    pub fn new(glob: GlobalToken) -> Result<NonNull<Self>, OutOfMemory> {
        let node = glob.allocator.lock().allocate()? as usize + HHDM_OFFSET;
        let node = unsafe { NonNull::new_unchecked(node as *mut Self) };
        unsafe {
            node.as_ptr().write(Self {
                next: None,
                len: 1,
                _marker: PhantomData,
            });
        }
        Ok(node)
    }
}

/// A cursor into a [`PageList<T>`].
pub struct Cursor<'a, T> {
    /// The current node.
    node: &'a mut NonNull<Node<T>>,
    /// The global token.
    glob: GlobalToken,
}

impl<'a, T> Cursor<'a, T> {
    /// Returns the number of elements that are part of the current node.
    #[inline]
    pub fn len(&self) -> usize {
        unsafe { self.node.as_ref().len }
    }

    /// Returns a pointer to the first element that's part of the current node.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        unsafe { self.node.as_ptr().add(1).cast() }
    }

    /// Returns a pointer to the first element that's part of the current node.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { self.node.as_ptr().add(1).cast() }
    }

    /// Assumes that the current node contains exactly `len` elements.
    ///
    /// # Safety
    ///
    /// That many elements must be part of the current node.
    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        unsafe { self.node.as_mut().len = len };
    }

    /// Returns whether the current node is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == Node::<T>::capacity()
    }

    /// Returns whether the current node has a next node.
    #[inline]
    pub fn has_next(&self) -> bool {
        unsafe { self.node.as_ref().next.is_some() }
    }

    /// Returns the element that are part of the current node.
    #[inline]
    pub fn current(&mut self) -> &[T] {
        let p = self.as_ptr();
        let l = self.len();

        unsafe { core::slice::from_raw_parts(p, l) }
    }

    /// Returns the element that are part of the current node.
    #[inline]
    pub fn current_mut(&mut self) -> &mut [T] {
        let p = self.as_mut_ptr();
        let l = self.len();

        unsafe { core::slice::from_raw_parts_mut(p, l) }
    }

    /// Returns the next node in the list, advancing the cursor.
    #[inline]
    pub fn advance(mut self) -> Option<Self> {
        if self.try_advance() {
            Some(self)
        } else {
            None
        }
    }

    /// Returns the next node in the list, advancing the cursor, but only
    /// if there is one.
    pub fn try_advance(&mut self) -> bool {
        if let Some(p) = unsafe { &mut self.node.as_mut().next } {
            self.node = p;
            true
        } else {
            false
        }
    }

    /// Returns the first node in the list that matches the provided predicate.
    pub fn try_find(&mut self, mut predicate: impl FnMut(&[T]) -> bool) -> bool {
        loop {
            if predicate(self.current()) {
                break true;
            }

            if !self.try_advance() {
                break false;
            }
        }
    }

    /// Removes an element from this cursor's current slice.
    ///
    /// This function preserves the order of the segments.
    ///
    /// # Safety
    ///
    /// This function assumes that `local_index` is in bounds.
    pub unsafe fn remove_unchecked(&mut self, local_index: usize) -> T {
        unsafe {
            let val = self.as_mut_ptr().add(local_index).read();

            core::ptr::copy(
                self.as_ptr().add(local_index).add(1),
                self.as_mut_ptr().add(local_index),
                self.len() - local_index - 1,
            );

            val
        }
    }

    /// Inserts an element into this cursor's current slice. If the slice is full,
    /// this function will return the last element.
    ///
    /// # Safety
    ///
    /// This function assumes that `local_index` is in bounds.
    pub unsafe fn insert_unchecked_no_spill(&mut self, local_index: usize, val: T) -> Option<T> {
        let ret = if self.is_full() {
            unsafe { Some(self.as_ptr().add(Node::<T>::capacity() - 1).read()) }
        } else {
            None
        };

        unsafe {
            core::ptr::copy(
                self.as_ptr().add(local_index),
                self.as_mut_ptr().add(local_index + 1),
                self.len() - local_index - 1,
            );

            self.as_mut_ptr().add(local_index).write(val);
        }

        ret
    }

    /// Inserts an element into this cursor's current slice. If the slice is full,
    /// the last element of this entry is spilled into the next entry.
    ///
    /// This function advances the cursor automatically to the entry in which elements
    /// are spilled.
    ///
    /// # Safety
    ///
    /// This function assumes that `local_index` is in bounds.
    pub unsafe fn insert_unchecked(
        &mut self,
        mut local_index: usize,
        mut val: T,
    ) -> Result<(), OutOfMemory> {
        let current = self;

        while let Some(spilled) = unsafe { current.insert_unchecked_no_spill(local_index, val) } {
            val = spilled;
            local_index = 0;

            if !current.has_next() {
                // We need to insert a new node after the current one.
                current.insert_after()?;
            }

            current.try_advance();
        }

        Ok(())
    }

    /// Returns the last node in the list.
    pub fn last(mut self) -> Self {
        while self.try_advance() {}
        self
    }

    /// Inserts a new node after the current one.
    pub fn insert_after(&mut self) -> Result<(), OutOfMemory> {
        let mut node = Node::new(self.glob)?;

        unsafe {
            node.as_mut().next = self.node.as_ref().next;
            let ret = self.node.as_mut().next.insert(node);
        }

        Ok(())
    }

    /// Attempts to find a node in the list that matches the provided predicate.
    pub fn find(mut self, mut predicate: impl FnMut(&[T]) -> bool) -> Option<Self> {
        loop {
            if predicate(self.current()) {
                break Some(self);
            }

            if !self.try_advance() {
                break None;
            }
        }
    }
}
