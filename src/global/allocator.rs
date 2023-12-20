use core::mem::MaybeUninit;

use x86_64::PhysAddr;

use crate::cpu::paging::HhdmToken;
use crate::utility::{BumpAllocator, FixedVec};

/// A memory allocator that keeps track of a list of free regions.
pub struct MemoryAllocator {
    /// We know that the HHDM has been initated already.
    _hhdm: HhdmToken,
    /// A list of the pages that are currently free and available for use.
    free_list: FixedVec<&'static mut [MaybeUninit<PhysAddr>]>,
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
        hhdm: HhdmToken,
        bootstrap_allocator: &mut BumpAllocator,
        capacity: usize,
    ) -> Result<Self, OutOfMemory> {
        let free_list_slice = bootstrap_allocator.allocate_slice(hhdm, capacity)?;

        Ok(Self {
            _hhdm: hhdm,
            free_list: FixedVec::new(free_list_slice),
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

/// An error returned when an allocation fails because the system is out of memory.
#[derive(Debug, Clone, Copy)]
pub struct OutOfMemory;
