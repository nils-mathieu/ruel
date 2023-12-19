//! This module provides the different structures and functions used to manage running processes.

use x86_64::{PageTable, PageTableIndex, PhysAddr, VirtAddr};

use crate::cpu::paging::{AddressSpace, AddressSpaceContext, HHDM_OFFSET, KERNEL_BIT};
use crate::global::{GlobalToken, OutOfMemory};

/// The last address that is part of userland.
pub const USERLAND_STOP: VirtAddr = 0x0000_7FFF_FFFF_FFFF;

/// A process that's running on the system.
pub struct Process {
    /// The address space of the process.
    pub address_space: AddressSpace<ASContext>,

    /// The current position of the instruction pointer of the process.
    pub ip: VirtAddr,
    /// The current position of the stack pointer of the process.
    pub sp: VirtAddr,
}

impl Process {
    /// Creates a new empty process.
    pub fn empty(glob: GlobalToken) -> Result<Self, OutOfMemory> {
        let mut address_space = AddressSpace::new(ASContext(glob))?;

        // Map the kernel into the address space.
        {
            let kernel_table =
                unsafe { &*((glob.address_space as usize + HHDM_OFFSET) as *const PageTable) };
            let process_table = unsafe { address_space.table_mut() };
            for i in PageTableIndex::iter() {
                if kernel_table[i].intersects(KERNEL_BIT) {
                    assert!(!process_table[i].is_present());
                    process_table[i] = kernel_table[i];
                }
            }
        }

        Ok(Self {
            address_space,
            ip: 0,
            sp: 0,
        })
    }
}

/// The address space context used for processes.
pub struct ASContext(GlobalToken);

unsafe impl AddressSpaceContext for ASContext {
    #[inline]
    fn allocate_page(&mut self) -> Result<PhysAddr, OutOfMemory> {
        self.0.allocator.lock().allocate()
    }

    #[inline]
    unsafe fn deallocate_page(&mut self, addr: PhysAddr) {
        unsafe { self.0.allocator.lock().deallocate(addr) }
    }

    #[inline]
    unsafe fn physical_to_virtual(&self, addr: PhysAddr) -> x86_64::VirtAddr {
        addr as usize + HHDM_OFFSET
    }
}
