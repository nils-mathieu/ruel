use core::ops::{Index, IndexMut};

use bitflags::bitflags;

use crate::IstIndex;

/// An Interrupt Descriptor Table.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Idt([GateDesc; 256]);

impl Index<u8> for Idt {
    type Output = GateDesc;

    #[inline]
    fn index(&self, index: u8) -> &Self::Output {
        unsafe { self.0.get_unchecked(index as usize) }
    }
}

impl IndexMut<u8> for Idt {
    #[inline]
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(index as usize) }
    }
}

impl Index<Exception> for Idt {
    type Output = GateDesc;

    #[inline]
    fn index(&self, index: Exception) -> &Self::Output {
        &self[index as u8]
    }
}

impl IndexMut<Exception> for Idt {
    #[inline]
    fn index_mut(&mut self, index: Exception) -> &mut Self::Output {
        &mut self[index as u8]
    }
}

/// An entry within an [`Idt`].
///
/// This is commonly called a "gate descriptor".
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct GateDesc([u64; 2]);

impl GateDesc {
    /// Creates a new gate descriptor with the provided base address.
    ///
    /// # Arguments
    ///
    /// - `base`: The base address of the interrupt handler.
    ///
    /// - `without_interrupts`: Whether the interrupt handler should be called with interrupts
    ///   disabled.
    ///
    /// - `ist`: The index of the interrupt stack table to use. Zero means no IST. Otherwise,
    ///   indices are one-based.
    ///
    /// - `dpl`: The highest privilege level that can call the interrupt handler.
    ///
    /// - `selector`: The code segment selector to use for the interrupt handler.
    ///
    /// - `present`: Whether the gate descriptor is actually present in the IDT.
    #[inline]
    pub const fn new(
        base: u64,
        without_interrupts: bool,
        ist: Option<IstIndex>,
        dpl: u8,
        selector: u16,
        present: bool,
    ) -> Self {
        assert!(dpl <= 3);

        let mut low = 0;
        let mut high = 0;

        if present {
            low |= 1 << 47;
        }

        if without_interrupts {
            low |= 0b1110 << 40;
        } else {
            low |= 0b1111 << 40;
        }

        low |= (base & 0xFFFF_0000) << 32 | (base & 0xFFFF);
        low |= match ist {
            Some(ist) => (ist as u64 + 1) << 32,
            None => 0,
        };
        low |= (dpl as u64) << 45;
        low |= (selector as u64) << 16;

        high |= base >> 32;

        Self([low, high])
    }
}

/// An exception that can occur on the CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exception {
    DivisionError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    X87FloatingPoint = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFloatingPoint = 19,
    VirtualizationException = 20,
    ControlProtection = 21,
    HypervisorInjection = 28,
    VmmCommunication = 29,
    SecurityException = 30,
}

/// The stack frame pushed by the CPU when an interrupt occurs.
#[repr(C)]
pub struct InterruptStackFrame {
    /// The instruction pointer at which the Interrupt Service Routine will return to.
    pub ip: u64,
    /// The code segment that the CPU switched from.
    pub cs: u64,
    /// The flags to restore when returning from the interrupt.
    pub flags: u64,
    /// The stack pointer to restore when returning from the interrupt.
    pub sp: u64,
    /// The stack segment to restore when returning from the interrupt.
    pub ss: u64,
}

bitflags! {
    /// The error code pushed by the CPU when a page fault occurs.
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct PageFaultError: u32 {
        /// Whether the page fault was caused by an access violation.
        ///
        /// Otherwise, it's because the page was not present in the page table.
        const PRESENT = 1 << 0;

        /// Whether the page fault was caused by a write operation.
        ///
        /// Otherwise, it was caused by a read operation.
        const WRITE = 1 << 1;

        /// Whether the page fault was caused by a write to a reserved bit.
        const RESERVED_WRITE = 1 << 2;

        /// Whether the page fault was caused in userland.
        ///
        /// Note that this does not necessarily mean that the error was a privilege violation.
        const USER = 1 << 3;

        /// Whether the page fault was caused by an instruction fetch on a non-executable page.
        const INSTRUCTION_FETCH = 1 << 4;
    }
}
