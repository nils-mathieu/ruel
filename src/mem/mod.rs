//! This module provides ways to manage memory on the system.

mod bump_allocator;
pub use self::bump_allocator::*;

/// A physical address.
pub type PhysAddr = u64;
