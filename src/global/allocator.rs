use core::alloc::Layout;
use core::mem::{align_of, size_of, MaybeUninit};

use x86_64::PhysAddr;

use crate::cpu::paging::HHDM_OFFSET;
use crate::utility::{ArrayVec, BumpAllocator};

/// A memory allocator that keeps track of a list of free regions.
pub struct MemoryAllocator {
    /// A list of the pages that are currently free and available for use.
    free_list: ArrayVec<&'static mut [MaybeUninit<PhysAddr>]>,
}

impl MemoryAllocator {
    /// Creates a new empty [`MemoryAllocator`] with the given capacity.
    ///
    /// The capacity is the maximum number of pages that can be managed by the allocator.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the global HHDM has been initialized.
    pub unsafe fn empty(
        bootstrap_allocator: &mut BumpAllocator,
        capacity: usize,
    ) -> Result<Self, OutOfMemory> {
        let free_list_slice = unsafe { &mut *allocate_slice(bootstrap_allocator, capacity)? };

        Ok(Self {
            free_list: ArrayVec::new(free_list_slice),
        })
    }

    /// Assumes that a given page is available for use.
    ///
    /// # Safety
    ///
    /// The allocator takes logical ownership of the physical page. Accessing it without having
    /// allocated it becomes unsafe and may cause conflicts with other parts of the system.
    pub unsafe fn assume_available(&mut self, page: PhysAddr) {
        debug_assert!(page & 0xFFF == 0);
        self.free_list.push(page);
    }

    /// Allocates a new page.
    pub fn allocate(&mut self) -> Result<PhysAddr, OutOfMemory> {
        self.free_list.pop().ok_or(OutOfMemory)
    }

    /// Deallocates a page that was previously allocated.
    ///
    /// # Safety
    ///
    /// The provided page must have been allocated previously by this allocator.
    pub unsafe fn deallocate(&mut self, page: PhysAddr) {
        debug_assert!(page & 0xFFF == 0);
        self.free_list.push(page);
    }
}

/// Allocates a slice using the provided allocator.
///
/// # Safety
///
/// The caller must ensure that the global HHDM has been initialized.
unsafe fn allocate_slice<T>(
    allocator: &mut BumpAllocator,
    len: usize,
) -> Result<*mut [T], OutOfMemory> {
    let align = align_of::<T>();
    let size = size_of::<T>()
        .checked_next_multiple_of(align)
        .ok_or(OutOfMemory)?;
    let layout = Layout::from_size_align(size * len, align).unwrap();

    let ptr = allocator.allocate(layout).map_err(|_| OutOfMemory)? as usize + HHDM_OFFSET;

    Ok(unsafe { core::ptr::slice_from_raw_parts_mut(ptr as *mut T, len) })
}

/// An error returned when an allocation fails because the system is out of memory.
#[derive(Debug, Clone, Copy)]
pub struct OutOfMemory;
