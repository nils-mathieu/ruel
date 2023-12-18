use crate::cpu::paging::{raw::*, FOUR_KIB, ONE_GIB, TWO_MIB};
use crate::mem::*;

use super::MappingError;

/// Represents an address space.
pub struct AddressSpace<C> {
    /// The context used to allocate and access pages.
    context: C,
    /// The root page table of the address space.
    root: PhysAddr,
}

impl<C: AddressSpaceContext> AddressSpace<C> {
    /// Creates a new [`AddressSpace`] with the provided context.
    pub fn new(mut context: C) -> Result<Self, OutOfMemory> {
        let root = context.allocate_page()?;

        unsafe {
            let root_ptr = context.physical_to_virtual(root) as *mut PageTable;
            core::ptr::write_bytes(root_ptr, 0x00, 1);
        }

        Ok(Self { context, root })
    }

    /// Returns the 4KiB page table entry for the provided virtual address.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to get the entry for.
    ///
    /// - `flags`: The flags to set on the entry's parent directory entries if they are not
    ///   present.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if the virtual address is not aligned
    /// to a 4KiB page.
    pub fn get_4kib_entry(&mut self, virt: VirtAddr, flags: u64) -> Result<&mut u64, MappingError> {
        debug_assert!(
            virt % FOUR_KIB == 0,
            "The virtual address is not aligned to a 4KiB page.",
        );

        let [p1, p2, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            let l3 = l4.get_directory(p4, flags, &mut self.context)?;
            let l2 = l3.get_directory(p3, flags, &mut self.context)?;
            let l1 = l2.get_directory(p2, flags, &mut self.context)?;
            Ok(&mut l1[p1])
        }
    }

    /// Returns the 2MiB page table entry for the provided virtual address.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to get the entry for.
    ///
    /// - `flags`: The flags to set on the entry's parent directory entries if they are not
    ///   present.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if the virtual address is not aligned
    /// to a 2MiB page.
    pub fn get_2mib_entry(&mut self, virt: VirtAddr, flags: u64) -> Result<&mut u64, MappingError> {
        debug_assert!(
            virt % TWO_MIB == 0,
            "The virtual address is not aligned to a 2MiB page.",
        );

        let [_, p2, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            let l3 = l4.get_directory(p4, flags, &mut self.context)?;
            let l2 = l3.get_directory(p3, flags, &mut self.context)?;
            Ok(&mut l2[p2])
        }
    }

    /// Returns the 1GiB page table entry for the provided virtual address.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to get the entry for.
    ///
    /// - `flags`: The flags to set on the entry's parent directory entries if they are not
    ///   present.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if the virtual address is not aligned
    /// to a 1GiB page.
    pub fn get_1gib_entry(&mut self, virt: VirtAddr, flags: u64) -> Result<&mut u64, MappingError> {
        debug_assert!(
            virt % ONE_GIB == 0,
            "The virtual address is not aligned to a 1GiB page.",
        );

        let [_, _, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            let l3 = l4.get_directory(p4, flags, &mut self.context)?;
            Ok(&mut l3[p3])
        }
    }

    /// Maps a 4KiB page to the provided physical address.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to map the page to.
    ///
    /// - `phys`: The physical address of the page to map.
    ///
    /// - `flags`: The flags to set on the page table entry.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if `virt` and `phys` are not aligned to a 4KiB page.
    pub fn map_4kib(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: u64,
    ) -> Result<(), MappingError> {
        debug_assert!(
            phys % FOUR_KIB as u64 == 0,
            "The physical address is not aligned to a 4KiB page."
        );

        let entry = self.get_4kib_entry(virt, flags)?;

        if *entry & PAGE_PRESENT != 0 {
            return Err(MappingError::AlreadyMapped);
        }

        *entry = phys | flags | PAGE_PRESENT;

        Ok(())
    }

    /// Maps a 2MiB page to the provided physical address.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to map the page to.
    ///
    /// - `phys`: The physical address of the page to map.
    ///
    /// - `flags`: The flags to set on the page table entry.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if `virt` and `phys` are not aligned to a 2MiB page.
    pub fn map_2mib(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: u64,
    ) -> Result<(), MappingError> {
        debug_assert!(
            phys % TWO_MIB as u64 == 0,
            "The physical address is not aligned to a 2MiB page."
        );

        let entry = self.get_2mib_entry(virt, flags)?;

        if *entry & PAGE_PRESENT != 0 {
            return Err(MappingError::AlreadyMapped);
        }

        *entry = phys | flags | PAGE_PRESENT | PAGE_SIZE;

        Ok(())
    }

    /// Maps a 1GiB page to the provided physical address.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to map the page to.
    ///
    /// - `phys`: The physical address of the page to map.
    ///
    /// - `flags`: The flags to set on the page table entry.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if `virt` and `phys` are not aligned to a 1GiB page.
    pub fn map_1gib(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: u64,
    ) -> Result<(), MappingError> {
        debug_assert!(
            phys % ONE_GIB as u64 == 0,
            "The physical address is not aligned to a 1GiB page."
        );

        let entry = self.get_1gib_entry(virt, flags)?;

        if *entry & PAGE_PRESENT != 0 {
            return Err(MappingError::AlreadyMapped);
        }

        *entry = phys | flags | PAGE_PRESENT | PAGE_SIZE;

        Ok(())
    }

    /// Maps the provided range of virtual addresses to the provided range of physical addresses.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address to start mapping at.
    ///
    /// - `phys`: The physical address to start mapping at.
    ///
    /// - `length`: The length of the range to map.
    ///
    /// - `flags`: The flags to set on the page table entries.
    ///
    /// # Panics
    ///
    /// In debug mode, this function panics if any of the input addresses are not properly
    /// aligned to a 4KiB page.
    pub fn map_range(
        &mut self,
        mut virt: VirtAddr,
        mut phys: PhysAddr,
        mut length: usize,
        flags: u64,
    ) -> Result<(), MappingError> {
        debug_assert!(
            virt % FOUR_KIB == 0,
            "The virtual address is not aligned to a 4KiB page.",
        );
        debug_assert!(
            phys % FOUR_KIB as u64 == 0,
            "The physical address is not aligned to a 4KiB page.",
        );
        debug_assert!(
            length % FOUR_KIB == 0,
            "The length is not a multiple of 4KiB.",
        );

        while length != 0 {
            if virt % ONE_GIB == 0 && phys % ONE_GIB as u64 == 0 && length >= ONE_GIB {
                self.map_1gib(virt, phys, flags)?;
                virt += ONE_GIB;
                phys += ONE_GIB as u64;
                length -= ONE_GIB;
            } else if virt % TWO_MIB == 0 && phys % TWO_MIB as u64 == 0 && length >= TWO_MIB {
                self.map_2mib(virt, phys, flags)?;
                virt += TWO_MIB;
                phys += TWO_MIB as u64;
                length -= TWO_MIB;
            } else {
                self.map_4kib(virt, phys, flags)?;
                virt += FOUR_KIB;
                phys += FOUR_KIB as u64;
                length -= FOUR_KIB;
            }
        }

        Ok(())
    }

    /// Leaks this [`AddressSpace`], exposing the underlying root L4 page table.
    #[inline]
    pub fn leak(self) -> PhysAddr {
        let ret = self.root;
        core::mem::forget(self);
        ret
    }
}

/// The context passed to an [`AddressSpace`] to describe how it should allocate new pages
/// of memory and how to access them.
///
/// # Safety
///
/// The pages returned by [`allocate_page`] must be unique. Their ownership is transferred to the
/// caller.
///
/// The [`physical_to_virtual`] function must return a valid virtual address to which the caller
/// can write safely (given they own the physical page in the first place).
///
/// [`allocate_page`]: AddressSpaceContext::allocate_page
pub unsafe trait AddressSpaceContext {
    /// Allocates a new page of memory.
    ///
    /// # Errors
    ///
    /// If the system is out of memory, this function returns an [`OutOfMemory`] error.
    fn allocate_page(&mut self) -> Result<PhysAddr, OutOfMemory>;

    /// Deallocates a page of memory previously allocated by [`allocate_page`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the page was previously allocated by [`allocate_page`].
    ///
    /// [`allocate_page`]: AddressSpaceContext::allocate_page
    unsafe fn deallocate_page(&mut self, addr: PhysAddr);

    /// Converts a physical address to a virtual address.
    ///
    /// # Safety
    ///
    /// The provided physical address must have been allocated by this context.
    unsafe fn physical_to_virtual(&self, addr: PhysAddr) -> VirtAddr;
}
