//! Defines the raw structures that the CPU expects for paging.

use core::ops::{Index, IndexMut};

use super::{AddressSpaceContext, MappingError};
use crate::mem::VirtAddr;

/// A page table.
#[derive(Clone, Copy)]
#[repr(align(4096))]
pub struct PageTable([u64; 512]);

impl PageTable {
    /// Returns the physical address of the page directory entry for the provided index.
    ///
    /// If the directory is not present, it is allocated.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the physical addresses that are part of the entries
    /// in this page table have been allocated by the provided context.
    pub unsafe fn get_directory(
        &mut self,
        index: PageTableIndex,
        flags: u64,
        context: &mut impl AddressSpaceContext,
    ) -> Result<&mut PageTable, MappingError> {
        if self[index] & PAGE_PRESENT == 0 {
            let table = context.allocate_page()?;

            unsafe {
                let table_ptr = context.physical_to_virtual(table) as *mut PageTable;
                core::ptr::write_bytes(table_ptr, 0x00, 1);

                self[index] = table | flags | PAGE_PRESENT;

                Ok(&mut *table_ptr)
            }
        } else if self[index] & PAGE_SIZE != 0 {
            Err(MappingError::AlreadyMapped)
        } else {
            self[index] |= flags;

            unsafe {
                let table = self[index] & PAGE_ADDRESS_MASK;
                let table_ptr = context.physical_to_virtual(table) as *mut PageTable;
                Ok(&mut *table_ptr)
            }
        }
    }
}

impl Index<PageTableIndex> for PageTable {
    type Output = u64;

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

/// The bit that indicates whether a page table entry is present or not.
pub const PAGE_PRESENT: u64 = 1 << 0;

/// The bit that indicates whether a page table entry represents a directory or an actual
/// entry mapping virtual addresses to physical addresses.
///
/// For the last level of the page table, this bit must be set to 0.
pub const PAGE_SIZE: u64 = 1 << 7;

/// Whether the page allows writes or not.
pub const PAGE_WRITE: u64 = 1 << 1;

/// Whether the page is global or not.
///
/// Global pages are not invalidated when the CR3 register is changed (i.e. when the address space
/// changes).
///
/// This is useful for pages that are shared between multiple address spaces, such as the kernel's
/// code and data.
pub const PAGE_GLOBAL: u64 = 1 << 8;

/// The mask that extracts the physical address from a page table entry.
pub const PAGE_ADDRESS_MASK: u64 = 0x0000_FFFF_FFFF_F000;

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

/// Aligns the provided value to the next multiple of the page size.
#[inline]
pub fn align_up(val: usize) -> usize {
    (val + 0xFFF) & !0xFFF
}

/// Aligns the provided value to the previous multiple of the page size.
#[inline]
pub fn align_down(val: usize) -> usize {
    val & !0xFFF
}
