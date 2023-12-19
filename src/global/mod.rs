//! This module contains the different parts of the global state of the kernel.

mod allocator;
pub use self::allocator::*;

use core::ops::Deref;

use x86_64::{PhysAddr, VirtAddr};

use crate::sync::{Mutex, OnceLock};

/// Stores the global state of the kernel.
pub struct Global {
    /// The memory allocator used by the kernel.
    pub allocator: Mutex<MemoryAllocator>,

    /// The physical address of the kernel in memory.
    pub kernel_physical_base: PhysAddr,
    /// The physical address of the L4 page table in memory.
    pub address_space: PhysAddr,
}

/// The global state of the kernel.
static GLOBAL: OnceLock<Global> = OnceLock::new();

/// Initializes the global state of the kernel.
///
/// # Panics
///
/// This function panics if the global state has already been initialized.
pub fn init(global: Global, kernel_stack_top: VirtAddr) -> GlobalToken {
    let mut called = false;
    GLOBAL.get_or_init(|| {
        called = true;
        unsafe { KERNEL_STACK_TOP = kernel_stack_top };
        global
    });

    assert!(
        called,
        "Attempted to initialize the global state of the kernel twice.",
    );

    GlobalToken(())
}

/// A token that can be used to access the global state of the kernel without checking if it has
/// been initialized.
#[derive(Clone, Copy)]
pub struct GlobalToken(());

impl GlobalToken {
    /// Attempts to get the global state of the kernel.
    ///
    /// # Panics
    ///
    /// This function panics if the global state has not been initialized yet.
    #[inline]
    pub fn get() -> Self {
        assert!(
            GLOBAL.is_initialized(),
            "Attempted to create a `GlobalToken` while the global state was not initialized",
        );
        Self(())
    }

    /// Returns whether the global state has been initialized already.
    #[inline]
    pub fn is_initialized() -> bool {
        GLOBAL.is_initialized()
    }
}

impl Deref for GlobalToken {
    type Target = Global;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { GLOBAL.get_unchecked() }
    }
}

/// Contains the stack pointer that the kernel should use when switching from kernel mode
/// to user mode.
///
/// This is required to be in a separate symbol because we need to access it in assembly when
/// handling system calls.
///
/// # Safety
///
/// Do not access mutably after initialization; do not access before initialization. Initialization
/// can be checked through the [`GlobalToken`] type.
pub static mut KERNEL_STACK_TOP: VirtAddr = 0;
