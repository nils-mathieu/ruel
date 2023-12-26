use x86_64::{PageTable, PageTableEntry, PageTableIndex, PhysAddr, VirtAddr};

use crate::global::OutOfMemory;
use crate::process::USERLAND_STOP;

/// The size of a 4KiB page.
pub const FOUR_KIB: usize = 4 * 1024;
/// The size of a 2MiB page.
pub const TWO_MIB: usize = 2 * 1024 * 1024;
/// The size of a 1GiB page.
pub const ONE_GIB: usize = 1024 * 1024 * 1024;
/// The size of a 4GiB page.
pub const FOUR_GIB: usize = 4 * 1024 * 1024 * 1024;

/// The offset of the higher-half direct map installed by the kernel during the booting process.
pub const HHDM_OFFSET: VirtAddr = 0xFFFF_8000_0000_0000;

/// A token that vouchers for the fact that the HHDM has been initiated.
///
/// When this token exists, physical addresses can be safely converted to a virtual address
/// by adding [`HHDM_OFFSET`] to their value.
#[derive(Clone, Copy)]
pub struct HhdmToken(());

impl HhdmToken {
    /// Creates a new [`HhdmToken`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the HHDM has been initiated.
    #[inline]
    pub const unsafe fn get() -> Self {
        Self(())
    }
}

/// An error that might occur while attempting to map some virtual memory to some physical memory.
#[derive(Debug, Clone, Copy)]
pub enum MappingError {
    /// A page could not be allocated.
    OutOfMemory,
    /// The virtual memory is already mapped.
    AlreadyMapped,
}

impl From<OutOfMemory> for MappingError {
    #[inline]
    fn from(_value: OutOfMemory) -> Self {
        MappingError::OutOfMemory
    }
}

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

    /// Returns the physical address of the L4 table of this address space.
    #[inline]
    pub fn l4_table(&self) -> PhysAddr {
        self.root
    }

    /// Returns the inner page table.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the page table is not modified in a way that would
    /// break invariants of the [`AddressSpace`].
    #[inline]
    pub unsafe fn table_mut(&mut self) -> &mut PageTable {
        unsafe { &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable) }
    }

    /// Attempts to translate the provided virtual address to a physical address.
    pub fn translate(&self, virt: VirtAddr) -> Option<PhysAddr> {
        let [p1, p2, p3, p4, _] = PageTableIndex::break_virtual_address(virt);
        let offset = (virt & 0xFFF) as u64;

        unsafe {
            let l4 = &*(self.context.physical_to_virtual(self.root) as *const PageTable);
            if !l4[p4].is_present() {
                return None;
            }
            let l3 = &*(self.context.physical_to_virtual(l4[p4].address()) as *const PageTable);
            if !l3[p3].is_present() {
                return None;
            }
            let l2 = &*(self.context.physical_to_virtual(l3[p3].address()) as *const PageTable);
            if !l2[p2].is_present() {
                return None;
            }
            let l1 = &*(self.context.physical_to_virtual(l2[p2].address()) as *const PageTable);
            if !l1[p1].is_present() {
                return None;
            }

            Some(l1[p1].address() + offset)
        }
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
    pub fn make_4kib_entry(
        &mut self,
        virt: VirtAddr,
        flags: PageTableEntry,
    ) -> Result<&mut PageTableEntry, MappingError> {
        debug_assert!(
            virt % FOUR_KIB == 0,
            "The virtual address is not aligned to a 4KiB page.",
        );

        let [p1, p2, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            let l3 = make_directory(l4, p4, flags, &mut self.context)?;
            let l2 = make_directory(l3, p3, flags, &mut self.context)?;
            let l1 = make_directory(l2, p2, flags, &mut self.context)?;

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
    pub fn make_2mib_entry(
        &mut self,
        virt: VirtAddr,
        flags: PageTableEntry,
    ) -> Result<&mut PageTableEntry, MappingError> {
        debug_assert!(
            virt % TWO_MIB == 0,
            "The virtual address is not aligned to a 2MiB page.",
        );

        let [_, p2, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            let l3 = make_directory(l4, p4, flags, &mut self.context)?;
            let l2 = make_directory(l3, p3, flags, &mut self.context)?;
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
    pub fn make_1gib_entry(
        &mut self,
        virt: VirtAddr,
        flags: PageTableEntry,
    ) -> Result<&mut PageTableEntry, MappingError> {
        debug_assert!(
            virt % ONE_GIB == 0,
            "The virtual address is not aligned to a 1GiB page.",
        );

        let [_, _, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            let l3 = make_directory(l4, p4, flags, &mut self.context)?;
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
        flags: PageTableEntry,
    ) -> Result<(), MappingError> {
        debug_assert!(
            phys % FOUR_KIB as u64 == 0,
            "The physical address is not aligned to a 4KiB page."
        );

        let entry = self.make_4kib_entry(virt, flags)?;

        if entry.is_present() {
            return Err(MappingError::AlreadyMapped);
        }

        *entry = PageTableEntry::from_address(phys) | flags | PageTableEntry::PRESENT;

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
        flags: PageTableEntry,
    ) -> Result<(), MappingError> {
        debug_assert!(
            phys % TWO_MIB as u64 == 0,
            "The physical address is not aligned to a 2MiB page."
        );

        let entry = self.make_2mib_entry(virt, flags)?;

        if entry.is_present() {
            return Err(MappingError::AlreadyMapped);
        }

        *entry = PageTableEntry::from_address(phys)
            | flags
            | PageTableEntry::PRESENT
            | PageTableEntry::HUGE_PAGE;

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
        flags: PageTableEntry,
    ) -> Result<(), MappingError> {
        debug_assert!(
            phys % ONE_GIB as u64 == 0,
            "The physical address is not aligned to a 1GiB page."
        );

        let entry = self.make_1gib_entry(virt, flags)?;

        if entry.is_present() {
            return Err(MappingError::AlreadyMapped);
        }

        *entry = PageTableEntry::from_address(phys)
            | flags
            | PageTableEntry::PRESENT
            | PageTableEntry::HUGE_PAGE;

        Ok(())
    }

    /// Returns the 4Kib entry for the provided virtual address.
    ///
    /// # Panics
    ///
    /// This function panics in debug builds if the provided virtual
    /// address is not properly aligned to a 4KiB page.
    ///
    /// # Returns
    ///
    /// If the provided virtual address is mapped to some physical address, returns
    /// the leaf entry for the 4KiB page. Otherwise, returns `None`.
    pub fn get_4kib_entry(&self, virt: VirtAddr) -> Result<&mut PageTableEntry, PageMiss> {
        debug_assert!(
            virt % FOUR_KIB == 0,
            "The virtual address is not properly aligned to a 4KiB page."
        );

        let [p1, p2, p3, p4, _] = PageTableIndex::break_virtual_address(virt);

        unsafe {
            let l4 = &mut *(self.context.physical_to_virtual(self.root) as *mut PageTable);
            if !l4[p4].is_present() || l4[p4].intersects(PageTableEntry::HUGE_PAGE) {
                return Err(PageMiss {
                    layer: MappingLayer::L4,
                    mapping: l4[p4].intersects(PageTableEntry::HUGE_PAGE),
                });
            }
            let l3 = &mut *(self.context.physical_to_virtual(l4[p4].address()) as *mut PageTable);
            if !l3[p3].is_present() || l3[p3].intersects(PageTableEntry::HUGE_PAGE) {
                return Err(PageMiss {
                    layer: MappingLayer::L3,
                    mapping: l4[p4].intersects(PageTableEntry::HUGE_PAGE),
                });
            }
            let l2 = &mut *(self.context.physical_to_virtual(l3[p3].address()) as *mut PageTable);
            if !l2[p2].is_present() || l2[p2].intersects(PageTableEntry::HUGE_PAGE) {
                return Err(PageMiss {
                    layer: MappingLayer::L2,
                    mapping: l4[p4].intersects(PageTableEntry::HUGE_PAGE),
                });
            }
            let l1 = &mut *(self.context.physical_to_virtual(l2[p2].address()) as *mut PageTable);
            if !l1[p1].is_present() {
                return Err(PageMiss {
                    layer: MappingLayer::L1,
                    mapping: false,
                });
            }

            Ok(&mut l1[p1])
        }
    }

    /// Attempts to unmap the provided 4KiB page.
    ///
    /// # Arguments
    ///
    /// - `virt`: The virtual address of the page to unmap.
    pub fn unmap_4kib(&mut self, virt: VirtAddr) -> Result<(), PageMiss> {
        let entry = self.get_4kib_entry(virt)?;

        *entry = PageTableEntry::empty();

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
        flags: PageTableEntry,
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

    /// Allocates the requested amount of memory, mapping it to the requested virtual addresses.
    ///
    /// The provided `callback` function can be used to initialize the allocated memory.
    ///
    /// # Panics
    ///
    /// In debug mode, this function panics if any of the input addresses are not properly
    /// aligned to a 4KiB page.
    pub fn allocate_range(
        &mut self,
        mut virt: VirtAddr,
        mut length: usize,
        flags: PageTableEntry,
        mut callback: impl FnMut(VirtAddr, *mut u8),
    ) -> Result<(), MappingError> {
        debug_assert!(
            virt % FOUR_KIB == 0,
            "The virtual address is not aligned to a 4KiB page.",
        );
        debug_assert!(
            length % FOUR_KIB == 0,
            "The length is not a multiple of 4KiB.",
        );

        while length != 0 {
            let phys = self.context.allocate_page()?;
            let dst = unsafe { self.context.physical_to_virtual(phys) as *mut u8 };

            self.map_4kib(virt, phys, flags)?;

            callback(virt, dst);

            virt += FOUR_KIB;
            length -= FOUR_KIB;
        }

        Ok(())
    }

    /// Unmaps the provided range of virtual addresses.
    ///
    /// # Panics
    ///
    /// This function panics in debug mode if any of the provided input
    /// arguments are not properly aligned to a 4KiB page boundary.
    ///
    /// # Errors
    ///
    /// This function returns `ALREADY_MAPPED` if any of the provided virtual addresses are not
    /// mapped.
    ///
    /// Note that in case of error, part of the requested range might have been properly
    /// unmapped.
    pub fn unmap_range(&mut self, mut virt: VirtAddr, mut length: usize) -> Result<(), PageMiss> {
        debug_assert!(
            virt % FOUR_KIB == 0,
            "The virtual address is not aligned to a 4KiB page.",
        );
        debug_assert!(
            length % FOUR_KIB == 0,
            "The length is not a multiple of 4KiB.",
        );

        while length != 0 {
            // FIXME: take larger mapping into account.
            self.unmap_4kib(virt)?;
            virt += FOUR_KIB;
            length -= FOUR_KIB;
        }

        Ok(())
    }

    /// Attempts to find an unmapped range of virtual addresses.
    ///
    /// # Panics
    ///
    /// This function panics in debug builds if `count` is not properly aligned
    /// to the page size.
    ///
    /// # Remarks
    ///
    /// This function only looks for valid memory within the common user-space
    /// area. (<= USERLAND_STOP)
    pub fn find_unmapped_range(&self, count: usize) -> Option<VirtAddr> {
        debug_assert!(
            count % FOUR_KIB == 0,
            "The length is not properly aligned to a 4KiB page."
        );

        const UPPER_BOUND: VirtAddr = USERLAND_STOP + 1;

        let mut virt = 0x1000;
        let mut count_so_far = 0;
        while virt < UPPER_BOUND && count_so_far < count {
            match self.get_4kib_entry(virt) {
                // The page is already mapped.
                // We can't use that.
                Ok(_) => virt += FOUR_KIB,
                Err(err) => {
                    match err.layer {
                        MappingLayer::L1 => {
                            // The only way to get here is if the last L1 layer
                            // is not mapped.
                            debug_assert!(!err.mapping);
                            count_so_far += FOUR_KIB;
                        }
                        MappingLayer::L2 => {
                            if err.mapping {
                                debug_assert!(virt % TWO_MIB == 0);

                                // A huge page is mapped.
                                count_so_far = 0;
                                virt += TWO_MIB;
                            } else {
                                // The last L2 layer is not mapped.
                                count_so_far += TWO_MIB;
                            }
                        }
                        MappingLayer::L3 => {
                            if err.mapping {
                                debug_assert!(virt % ONE_GIB == 0);

                                // A huge page is mapped.
                                count_so_far = 0;
                                virt += ONE_GIB;
                            } else {
                                // The last L3 layer is not mapped.
                                count_so_far += ONE_GIB;
                            }
                        }
                        MappingLayer::L4 => {
                            if err.mapping {
                                // Not possible because five-level paging
                                // is not enabled.
                                unreachable!();
                            } else {
                                // The last L4 layer is not mapped.
                                count_so_far += FOUR_GIB;
                            }
                        }
                    }
                }
            }
        }

        if count_so_far >= count {
            Some(virt)
        } else {
            None
        }
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

/// Returns the physical address of the page directory entry for the provided index.
///
/// If the directory is not present, it is allocated.
///
/// # Safety
///
/// The caller must ensure that the physical addresses that are part of the entries
/// in this page table have been allocated by the provided context.
unsafe fn make_directory<'a>(
    table: &'a mut PageTable,
    index: PageTableIndex,
    flags: PageTableEntry,
    context: &mut impl AddressSpaceContext,
) -> Result<&'a mut PageTable, MappingError> {
    if !table[index].is_present() {
        let new_table = context.allocate_page()?;

        unsafe {
            let table_ptr = context.physical_to_virtual(new_table) as *mut PageTable;
            core::ptr::write_bytes(table_ptr, 0x00, 1);

            table[index] =
                PageTableEntry::from_address(new_table) | flags | PageTableEntry::PRESENT;

            Ok(&mut *table_ptr)
        }
    } else if table[index].intersects(PageTableEntry::HUGE_PAGE) {
        Err(MappingError::AlreadyMapped)
    } else {
        update_parent(&mut table[index], flags);

        unsafe {
            let table = table[index].address();
            let table_ptr = context.physical_to_virtual(table) as *mut PageTable;
            Ok(&mut *table_ptr)
        }
    }
}

/// Updates the flags of `parent` such that it keeps the same semantics as before, but with that
/// of the child entry added.
fn update_parent(parent: &mut PageTableEntry, child: PageTableEntry) {
    debug_assert!(!parent.intersects(PageTableEntry::HUGE_PAGE));
    debug_assert!(parent.intersects(PageTableEntry::PRESENT));

    // const PRESERVED_FLAGS: PageTableEntry =
    //     PageTableEntry::PRESENT.union(PageTableEntry::PAGE_ADDRESS_MASK);
    // const AND_FLAGS: PageTableEntry = PageTableEntry::NO_EXECUTE.union(PageTableEntry::GLOBAL);
    // const OR_FLAGS: PageTableEntry = PageTableEntry::WRITABLE
    //     .union(PageTableEntry::USER_ACCESSIBLE)
    //     .union(KERNEL_BIT)
    //     .union(NOT_OWNED_BIT);

    // let child_and = AND_FLAGS & child;
    // let parent_and = AND_FLAGS & *parent;
    // let child_or = OR_FLAGS & child;
    // let parent_or = OR_FLAGS & *parent;
    // let parent_preserved = PRESERVED_FLAGS & *parent;

    // *parent = parent_preserved | (parent_and & child_and) | (parent_or | child_or);

    *parent |= child;
}

/// A bit that's set for kernel pages. Used when copying the kernel address space to a process.
pub const KERNEL_BIT: PageTableEntry = PageTableEntry::OS_BIT_9;

/// A bit that's set for pages that are *not* owned by the process, meaning that they must not be
/// given back to the kernel when the process is destroyed.
pub const NOT_OWNED_BIT: PageTableEntry = PageTableEntry::OS_BIT_10;

/// A possible mapping layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingLayer {
    L4,
    L3,
    L2,
    L1,
}

/// A possible way to miss a mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMiss {
    /// The layer at which the miss occured.
    pub layer: MappingLayer,
    /// Whether the page was present.
    ///
    /// If set, the page was mapping a huge page (or a regular mapping).
    ///
    /// If clear, the page was not present.
    pub mapping: bool,
}
