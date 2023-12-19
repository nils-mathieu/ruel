use core::arch::asm;

use bitflags::bitflags;

use crate::{rdmsr, wrmsr, SegmentSelector};

/// Writes to the CS register.
///
/// # Safety
///
/// Writing arbitrary values to the CS register can compromise memory safety.
pub unsafe fn write_cs(val: SegmentSelector) {
    unsafe {
        // The code segment selector cannot be modified with a simple move.
        // The common workaround is to push the selector to the stack and then pop it into the
        // `cs` register using the RETFQ instruction.
        asm!(
            "
            push {}
            lea {tmp}, [2f + rip]
            push {tmp}
            retfq
        2:
            ",
            in(reg) val.bits() as usize, // this is important, the selector is 16-bit wide, but we need to push a full word on the stack.
            tmp = lateout(reg) _,
            options(preserves_flags, nomem),
        );
    }
}

macro_rules! read_write_selector_reg {
    ($upper:literal, $lower:literal, $write_fn:ident, $read_fn:ident) => {
        #[doc = ::core::concat!("Reads the ", $upper, " register.")]
        #[inline]
        pub fn $read_fn() -> SegmentSelector {
            let mut val: u16;

            unsafe {
                asm!(
                    ::core::concat!("mov {:x}, ", $lower),
                    out(reg) val,
                    options(nomem, nostack, preserves_flags)
                );
            }

            SegmentSelector::from_bits(val)
        }

        #[doc = ::core::concat!("Writes to the ", $upper, " register.")]
        ///
        /// # Safety
        ///
        #[doc = ::core::concat!("Writing arbitrary values to the ", $upper, " register can compromise memory safety.")]
        #[inline]
        pub unsafe fn $write_fn(val: SegmentSelector) {
            unsafe {
                asm!(
                    ::core::concat!("mov ", $lower, ", {:x}"),
                    in(reg) val.bits(),
                    options(nomem, nostack, preserves_flags)
                );
            }
        }
    };
}

read_write_selector_reg!("SS", "ss", write_ss, read_ss);
read_write_selector_reg!("DS", "ds", write_ds, read_ds);
read_write_selector_reg!("ES", "es", write_es, read_es);
read_write_selector_reg!("FS", "fs", write_fs, read_fs);
read_write_selector_reg!("GS", "gs", write_gs, read_gs);

/// The address of the Extended Feature Enable Register (EFER) MSR.
pub const EFER: u32 = 0xC000_0080;

bitflags! {
    /// The flags of the Extended Feature Enable Register (EFER) MSR.
    #[derive(Default, Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct Efer: u64 {
        /// Enables the SYSCALL/SYSRET instructions in 64-bit mode.
        const SYSCALL = 1 << 0;
        /// Enables the NXE bit in the page tables.
        const NO_EXECUTE = 1 << 11;
        /// Enables the long mode.
        const LONG_MODE = 1 << 8;
    }
}

impl Efer {
    /// Reads the content of the Extended Feature Enable Register (EFER) MSR.
    #[inline]
    pub fn read() -> Self {
        unsafe { Self::from_bits_retain(rdmsr(EFER)) }
    }

    /// Writes to the Extended Feature Enable Register (EFER) MSR.
    #[inline]
    pub fn write(self) {
        unsafe { wrmsr(EFER, self.bits()) }
    }
}

/// The LSTAR MSR address.
pub const LSTAR: u32 = 0xC000_0082;

/// The STAR MSR address.
pub const STAR: u32 = 0xC000_0081;
