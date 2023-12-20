//! This module provides the different structures and functions used to manage running processes.

use core::ptr::NonNull;

use ruel_sys::{ProcessConfig, WakeUp};
use x86_64::{PageTable, PageTableIndex, PhysAddr, VirtAddr};

use crate::cpu::paging::{AddressSpace, AddressSpaceContext, HHDM_OFFSET, KERNEL_BIT};
use crate::global::{GlobalToken, OutOfMemory};

use self::io_states::IoStates;

mod io_states;

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
    ///
    /// # Safety
    ///
    /// The created [`ProcessPtr<T>`] must be destroyed before the process is.
    #[inline]
    pub const unsafe fn new(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }

    /// Reads the value from the pointer.
    ///
    /// # Remarks
    ///
    /// This function can race with the process if it has some other threads running
    /// concurrently. When that happens, it's undefined behavior. There's no good way to
    /// solve this, but since the pointer has no provenance, the compiler won't be able to
    /// optimize it in a way that breaks the program.
    ///
    /// What we *do* need to care about, however, is to check the value just before we actually
    /// read it.
    #[inline]
    pub fn as_ref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

/// When a process is currently waiting for some condition to be met, this type stores which
/// conditions are being waited on.
pub enum SleepingState {
    /// Pointer to a userspace buffer.
    InProcess(ProcessPtr<[WakeUp]>),
    /// Inline-waiting on a condition to avoid allocating memory.
    InKernel(WakeUp),
}

impl SleepingState {
    /// Returns a reference to the inner value.
    #[inline]
    pub fn as_ref(&self) -> &[WakeUp] {
        match self {
            Self::InProcess(ptr) => ptr.as_ref(),
            Self::InKernel(wake_up) => core::slice::from_ref(wake_up),
        }
    }
}

/// A process that's running on the system.
pub struct Process {
    /// The address space of the process.
    pub address_space: AddressSpace<ASContext>,
    /// The current state of the process.
    pub registers: Registers,
    /// The local I/O state reported to the process.
    pub io_states: IoStates,
    /// The state of the process.
    pub sleeping: Option<SleepingState>,
    /// The configuration flags of the process.
    pub config: ProcessConfig,
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
            io_states: IoStates::empty(),
            config: ProcessConfig::empty(),
        })
    }

    /// Ticks the process once.
    pub fn tick(&mut self) {
        let mut woken_up = false;

        if let Some(sleeping) = &mut self.sleeping {
            for wake_up in sleeping.as_ref() {
                match wake_up.tag() {
                    ruel_sys::WakeUpTag::PS2_KEYBOARD => {
                        if self.io_states.ps2_keyboard.total_len() > 0 {
                            woken_up = true;
                        }
                    }
                    _ => {
                        // TODO: properly propagate the error to the process.
                    }
                }
            }
        }

        if woken_up {
            self.sleeping = None;
        }
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
