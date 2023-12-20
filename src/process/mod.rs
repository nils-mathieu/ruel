//! This module provides the different structures and functions used to manage running processes.

use core::ptr::NonNull;

use ruel_sys::WakeUp;
use x86_64::{PageTable, PageTableIndex, PhysAddr, VirtAddr};

use crate::cpu::paging::{AddressSpace, AddressSpaceContext, HHDM_OFFSET, KERNEL_BIT};
use crate::global::{GlobalToken, OutOfMemory};

/// The last address that is part of userland.
pub const USERLAND_STOP: VirtAddr = 0x0000_7FFF_FFFF_FFFF;

/// The registers of a paused process.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Registers {
    pub rip: usize,
    pub rsp: usize,
    pub rbp: usize,
    pub rdi: usize,
}

impl Registers {
    pub const RIP_INDEX: usize = 0;
    pub const RSP_INDEX: usize = 1;
    pub const RBP_INDEX: usize = 2;
    pub const RDI_INDEX: usize = 3;
}

/// A pointer into a process's address space.
pub struct ProcessPtr<T: ?Sized>(NonNull<T>);

unsafe impl<T: ?Sized + Send> Send for ProcessPtr<T> {}
unsafe impl<T: ?Sized + Sync> Sync for ProcessPtr<T> {}

impl<T: ?Sized> ProcessPtr<T> {
    /// Creates a new [`ProcessPtr<T>`] instance.
    #[inline]
    pub const fn new(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }

    // /// Returns a reference to the inner value.
    // ///
    // /// # Safety
    // ///
    // /// The process that owns this pointer must be still around.
    // #[inline]
    // pub unsafe fn as_ref(&self) -> &T {
    //     unsafe { self.0.as_ref() }
    // }

    /// Returns a mutable reference to the inner value.
    ///
    /// # Safety
    ///
    /// The process that owns this pointer must be still around, and the memory it references
    /// must not be shared with another eventually running process.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

/// When a process is currently waiting for some condition to be met, this type stores which
/// conditions are being waited on.
pub struct SleepingState {
    /// The conditions that the process is waiting on.
    pub wake_ups: ProcessPtr<[WakeUp]>,
}

/// A process that's running on the system.
pub struct Process {
    /// The address space of the process.
    pub address_space: AddressSpace<ASContext>,

    /// The current state of the process.
    pub registers: Registers,

    /// The state of the process.
    pub sleeping: Option<SleepingState>,
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
            registers: Registers::default(),
            sleeping: None,
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
