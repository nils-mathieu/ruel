use core::ops::{Index, IndexMut};

/// An Interrupt Descriptor Table.
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

/// An entry within an [`Idt`].
///
/// This is commonly called a "gate descriptor".
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
        ist: u8,
        dpl: u8,
        selector: u16,
        present: bool,
    ) -> Self {
        assert!(ist < 8);
        assert!(dpl <= 3);

        let mut low = 0;
        let mut high = 0;

        if present {
            low |= GATE_PRESENT;
        }

        if without_interrupts {
            low |= GATE_INTERRUPT;
        } else {
            low |= GATE_TRAP;
        }

        low |= (base & 0xFFFF_0000) << 32 | (base & 0xFFFF);
        low |= (ist as u64) << 32;
        low |= (dpl as u64) << 45;
        low |= (selector as u64) << 16;

        high |= base >> 32;

        Self([low, high])
    }
}

/// Whether the gate descriptor is actually present in the IDT.
const GATE_PRESENT: u64 = 1 << 47;
/// Whether the gate descriptor is a trap gate.
const GATE_TRAP: u64 = 0b1111 << 40;
/// Whether the gate descriptor is an interrupt gate.
const GATE_INTERRUPT: u64 = 0b1110 << 40;

/// The stack frame pushed by the CPU when an interrupt occurs.
#[repr(C)]
pub struct InterruptStackFrame {
    pub ip: u64,
    pub cs: u64,
    pub flags: u64,
    pub sp: u64,
    pub ss: u64,
}

pub const VECNBR_DIVISION_ERROR: u8 = 0;
pub const VECNBR_DEBUG: u8 = 1;
pub const VECNBR_NON_MASKABLE_INTERRUPT: u8 = 2;
pub const VECNBR_BREAKPOINT: u8 = 3;
pub const VECNBR_OVERFLOW: u8 = 4;
pub const VECNBR_BOUND_RANGE_EXCEEDED: u8 = 5;
pub const VECNBR_INVALID_OPCODE: u8 = 6;
pub const VECNBR_DEVICE_NOT_AVAILABLE: u8 = 7;
pub const VECNBR_DOUBLE_FAULT: u8 = 8;
pub const VECNBR_INVALID_TSS: u8 = 10;
pub const VECNBR_SEGMENT_NOT_PRESENT: u8 = 11;
pub const VECNBR_STACK_SEGMENT_FAULT: u8 = 12;
pub const VECNBR_GENERAL_PROTECTION_FAULT: u8 = 13;
pub const VECNBR_PAGE_FAULT: u8 = 14;
pub const VECNBR_X87_FLOATING_POINT_EXCEPTION: u8 = 16;
pub const VECNBR_ALIGNMENT_CHECK: u8 = 17;
pub const VECNBR_MACHINE_CHECK: u8 = 18;
pub const VECNBR_SIMD_FLOATING_POINT_EXCEPTION: u8 = 19;
pub const VECNBR_VIRTUALIZATION_EXCEPTION: u8 = 20;
pub const VECNBR_CONTROL_PROTECTION_EXCEPTION: u8 = 21;
pub const VECNBR_HYPERVISOR_INJECTION_EXCEPTION: u8 = 28;
pub const VECNBR_VMM_COMMUNICATION_EXCEPTION: u8 = 29;
pub const VECNBR_SECURITY_EXCEPTION: u8 = 30;
