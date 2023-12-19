//! This module contains the different parts of the global state of the kernel.

mod allocator;
pub use self::allocator::*;

use core::ops::Deref;

use x86_64::PhysAddr;

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
pub fn init(global: Global) -> GlobalToken {
    GLOBAL
        .set(global)
        .ok()
        .expect("the global state has already been initialized");
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
}

impl Deref for GlobalToken {
    type Target = Global;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { GLOBAL.get_unchecked() }
    }
}
