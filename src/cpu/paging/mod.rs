use crate::mem::{OutOfMemory, VirtAddr};

pub mod raw;

mod address_space;
pub use self::address_space::*;

/// The size of a 4KiB page.
pub const FOUR_KIB: usize = 4 * 1024;
/// The size of a 2MiB page.
pub const TWO_MIB: usize = 2 * 1024 * 1024;
/// The size of a 1GiB page.
pub const ONE_GIB: usize = 1024 * 1024 * 1024;

/// The offset of the higher-half direct map installed by the kernel during the booting process.
pub const HHDM_OFFSET: VirtAddr = 0xFFFF_8000_0000_0000;

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
