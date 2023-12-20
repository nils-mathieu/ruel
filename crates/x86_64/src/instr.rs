use core::arch::asm;

use crate::SegmentSelector;

/// A pointer to a table (such as the GDT or IDT).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TablePtr {
    pub limit: u16,
    pub base: *const (),
}

/// Uses the LGDT instruction.
///
/// # Safety
///
/// Loading an invalid IDT can compromise memory safety.
#[inline]
pub unsafe fn lgdt(table: *const TablePtr) {
    unsafe {
        asm!(
            "lgdt [{}]",
            in(reg) table,
            options(nostack, readonly, preserves_flags),
        );
    }
}

/// Uses the LIDT instruction.
///
/// # Safety
///
/// Loading an invalid IDT can compromise memory safety.
#[inline]
pub unsafe fn lidt(table: *const TablePtr) {
    unsafe {
        asm!(
            "lidt [{}]",
            in(reg) table,
            options(nostack, readonly, preserves_flags),
        );
    }
}

/// Loads the currently active GDT.
#[inline]
pub fn sgdt() -> TablePtr {
    let mut table = TablePtr {
        limit: 0,
        base: core::ptr::null(),
    };

    unsafe {
        asm!(
            "sgdt [{}]",
            in(reg) &mut table,
            options(nomem, nostack, preserves_flags),
        );
    }

    table
}

/// Loads the currently active IDT.
#[inline]
pub fn sidt() -> TablePtr {
    let mut table = TablePtr {
        limit: 0,
        base: core::ptr::null(),
    };

    unsafe {
        asm!(
            "sidt [{}]",
            in(reg) &mut table,
            options(nomem, nostack, preserves_flags),
        );
    }

    table
}

/// Uses the LTR instruction.
///
/// # Safety
///
/// Loading an invalid TSS can compromise memory safety.
#[inline]
pub unsafe fn ltr(selector: SegmentSelector) {
    unsafe {
        asm!(
            "ltr {:x}",
            in(reg) selector.bits(),
            options(nostack, nomem, preserves_flags),
        );
    }
}

/// Executes the HLT instruction.
pub fn hlt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Executes the CLI instruction.
#[inline]
pub fn cli() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Executes the STI instruction.
#[inline]
pub fn sti() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Writes a byte to the provided I/O port.
///
/// # Safety
///
/// Writing to arbitrary I/O ports can compromise memory safety.
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Reads a byte from the provided I/O port.
///
/// # Safety
///
/// Reading from arbitrary I/O ports can compromise memory safety.
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;

    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Reads the contents of the provided model-specific register (MSR).
///
/// # Safety
///
/// Reading from arbitrary MSRs can compromise memory safety.
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let mut low: u32;
    let mut high: u32;

    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }

    ((high as u64) << 32) | (low as u64)
}

/// Writes the contents to the provided model-specific register (MSR).
///
/// # Safety
///
/// Writing to arbitrary MSRs can compromise memory safety.
pub unsafe fn wrmsr(msr: u32, val: u64) {
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") val as u32,
            in("edx") (val >> 32) as u32,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Invokes a software interrupt handler.
///
/// # Safety
///
/// Invoking an invalid interrupt handler can compromise memory safety.
#[inline]
pub unsafe fn int<const N: u8>() {
    unsafe {
        asm!(
            "int {}",
            const N as usize,
            options(nomem, nostack, preserves_flags),
        );
    }
}
