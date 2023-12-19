//! This module provides ways to manage memory on the system.

mod bump_allocator;
pub use self::bump_allocator::*;

mod allocator;
pub use self::allocator::*;

/// An error returned when an allocation fails because the system is out of memory.
#[derive(Debug, Clone, Copy)]
pub struct OutOfMemory;
