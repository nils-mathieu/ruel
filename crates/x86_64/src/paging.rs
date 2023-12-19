use core::ops::{Index, IndexMut};

use bitflags::bitflags;

use crate::{PhysAddr, VirtAddr};

/// A page table.
#[derive(Clone, Copy)]
#[repr(align(4096))]
pub struct PageTable([PageTableEntry; 512]);

impl Index<PageTableIndex> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: PageTableIndex) -> &Self::Output {
        unsafe { self.0.get_unchecked(index.index()) }
    }
}

impl IndexMut<PageTableIndex> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: PageTableIndex) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(index.index()) }
    }
}

/// An index within a [`PageTable`].
///
/// This allows accessing a [`PageTable`] with no bound checks.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    /// Creates a new [`PageTableIndex`] from the given index.
    pub fn break_virtual_address(virt: VirtAddr) -> [Self; 5] {
        [
            Self(((virt >> 12) & 0o777) as u16),
            Self(((virt >> 21) & 0o777) as u16),
            Self(((virt >> 30) & 0o777) as u16),
            Self(((virt >> 39) & 0o777) as u16),
            Self(((virt >> 48) & 0o777) as u16),
        ]
    }

    /// Returns the index that this [`PageTableIndex`] represents.
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

bitflags! {
    /// An entry in a [`PageTable`].
    ///
    /// # Remarks
    ///
    /// This type is actually responsible for representing not only page table entries. This
    /// includes the final virtual-to-physical mappings, but also directory and directory pointers.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct PageTableEntry: u64 {
        /// Whether the entry is present.
        const PRESENT = 1 << 0;

        /// Whether the entry represents a virtual-to-physical mapping instead of a directory.
        ///
        /// This is only valid when the entry is not the last level of the page table, and the
        /// size of the final mapping depends on the depth within the page table.
        const SIZE = 1 << 7;

        /// Whether the page can be written to.
        const WRITABLE = 1 << 1;

        /// Whether the page can be accessed by code running at ring 3.
        const USER_ACCESSIBLE = 1 << 2;

        /// Whether the page can be executed.
        const NO_EXECUTE = 1 << 63;

        /// A mask that includes the bits of the address part of the entry.
        ///
        /// # Remarks
        ///
        /// Depending on the level of the entry, the subset of those bits might change.
        const PAGE_ADDRESS_MASK = 0x000F_FFFF_FFFF_F000;

        /// Whether the page is global or not.
        ///
        /// Global pages are not invalidated when the CR3 register is changed (i.e. when the address space
        /// changes).
        ///
        /// This is useful for pages that are shared between multiple address spaces, such as the kernel's
        /// code and data.
        const GLOBAL = 1 << 8;
    }
}

impl PageTableEntry {
    /// Creates a new [`PageTableEntry`] from the provided physical address.
    ///
    /// # Panics
    ///
    /// In debug mode, this function panics if the provided physical address
    /// is not aligned to a page boundary. Note that this check is not
    /// necessarily sufficient if the page is a huge page (with the SIZE bit set).
    #[inline]
    pub const fn from_address(phys: PhysAddr) -> Self {
        debug_assert!(phys & !Self::PAGE_ADDRESS_MASK.bits() == 0);
        Self::from_bits_retain(phys)
    }

    /// Returns whether the entry is present or not.
    #[inline]
    pub const fn is_present(self) -> bool {
        self.intersects(Self::PRESENT)
    }

    /// Returns the physical address that this entry points to.
    #[inline]
    pub const fn address(self) -> PhysAddr {
        self.bits() & Self::PAGE_ADDRESS_MASK.bits()
    }
}

/// Aligns the provided value to the next multiple of the page size.
#[inline]
pub fn page_align_up(val: usize) -> usize {
    (val + 0xFFF) & !0xFFF
}

/// Aligns the provided value to the previous multiple of the page size.
#[inline]
pub fn page_align_down(val: usize) -> usize {
    val & !0xFFF
}
