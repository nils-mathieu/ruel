//! Defines the structures used by the x86_64 CPU architecture, as well as functions to call
//! functions that are specific to it.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

mod gdt;
pub use self::gdt::*;

mod reg;
pub use self::reg::*;

mod instr;
pub use self::instr::*;

mod idt;
pub use self::idt::*;

mod paging;
pub use self::paging::*;

/// A privilege level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Ring {
    /// Ring 0
    Zero = 0b00,
    /// Ring 1
    One = 0b01,
    /// Ring 2
    Two = 0b10,
    /// Ring 3
    Three = 0b11,
}

impl Ring {
    /// Creates a new [`Ring`] instance from the provided raw value.
    ///
    /// # Safety
    ///
    /// The raw value must be in the range `0..=3`.
    #[inline]
    pub const unsafe fn from_raw(r: u8) -> Self {
        unsafe { core::mem::transmute(r) }
    }
}

/// A virtual address.
pub type VirtAddr = usize;

/// A physical address.
pub type PhysAddr = u64;
