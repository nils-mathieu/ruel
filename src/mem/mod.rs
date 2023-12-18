//! This module provides ways to manage memory on the system.

mod bump_allocator;
pub use self::bump_allocator::*;

/// A physical address.
pub type PhysAddr = u64;

/// A virtual address.
pub type VirtAddr = usize;

/// An error returned when an allocation fails because the system is out of memory.
#[derive(Debug, Clone, Copy)]
pub struct OutOfMemory;
