//! This module mainly provides the [`init`] function used to initialize the kernel's IDT.
//!
//! The implementation of the Interrupt Service Routines (ISRs) are located in the [`handlers`]
//! module.

use core::alloc::Layout;
use core::arch::asm;
use core::mem::size_of;

use x86_64::{Exception, GateDesc, Idt};

use super::gdt::{DOUBLE_FAULT_IST_INDEX, KERNEL_CODE_SELECTOR};
use super::paging::HHDM_OFFSET;
use crate::log;
use crate::mem::{BumpAllocator, OutOfMemory};

mod handlers;

/// Initializes the kernel's IDT.
pub fn init(bootstrap_allocator: &mut BumpAllocator) -> Result<(), OutOfMemory> {
    log::trace!("Initializing the IDT...");

    let idt_phys_addr = bootstrap_allocator.allocate(Layout::new::<Idt>())?;
    let idt = unsafe { &mut *((idt_phys_addr as usize + HHDM_OFFSET) as *mut Idt) };

    idt[Exception::DivisionError] = trap_gate(handlers::division_error as usize);
    idt[Exception::Debug] = trap_gate(handlers::debug as usize);
    idt[Exception::NonMaskableInterrupt] = int_gate(handlers::non_maskable_interrupt as usize);
    idt[Exception::Breakpoint] = trap_gate(handlers::breakpoint_handler as usize);
    idt[Exception::Overflow] = trap_gate(handlers::overflow as usize);
    idt[Exception::BoundRangeExceeded] = trap_gate(handlers::bound_range_exceeded as usize);
    idt[Exception::InvalidOpcode] = trap_gate(handlers::invalid_opcode as usize);
    idt[Exception::DeviceNotAvailable] = trap_gate(handlers::device_not_available as usize);
    idt[Exception::DoubleFault] = double_fault_gate();
    idt[Exception::InvalidTss] = trap_gate(handlers::invalid_tss as usize);
    idt[Exception::SegmentNotPresent] = trap_gate(handlers::segment_not_present as usize);
    idt[Exception::StackSegmentFault] = trap_gate(handlers::stack_segment_fault as usize);
    idt[Exception::GeneralProtectionFault] = trap_gate(handlers::general_protection_fault as usize);
    idt[Exception::PageFault] = trap_gate(handlers::page_fault as usize);
    idt[Exception::X87FloatingPoint] = trap_gate(handlers::x87_floating_point as usize);
    idt[Exception::AlignmentCheck] = trap_gate(handlers::alignment_check as usize);
    idt[Exception::MachineCheck] = trap_gate(handlers::machine_check as usize);
    idt[Exception::SimdFloatingPoint] = trap_gate(handlers::simd_floating_point as usize);
    idt[Exception::VirtualizationException] = trap_gate(handlers::virtualization as usize);
    idt[Exception::ControlProtection] = trap_gate(handlers::control_protection as usize);
    idt[Exception::HypervisorInjection] = trap_gate(handlers::hypervisor_injection as usize);
    idt[Exception::VmmCommunication] = trap_gate(handlers::vmm_communication as usize);
    idt[Exception::SecurityException] = trap_gate(handlers::security_exception as usize);

    log::trace!("Loading the IDT...");

    #[repr(C, packed)]
    struct Idtr {
        limit: u16,
        base: usize,
    }

    let idtr = Idtr {
        limit: size_of::<Idt>() as u16 - 1,
        base: idt as *const _ as usize,
    };

    unsafe {
        asm!("lidt [{}]", in(reg) &idtr, options(nostack, readonly, preserves_flags));
    }

    Ok(())
}

/// Creates a new trap gate.
fn trap_gate(handler: usize) -> GateDesc {
    GateDesc::new(
        handler as u64,
        false,
        None,
        0,
        KERNEL_CODE_SELECTOR.bits(),
        true,
    )
}

/// Creates a new interrupt gate.
fn int_gate(handler: usize) -> GateDesc {
    GateDesc::new(
        handler as u64,
        true,
        None,
        0,
        KERNEL_CODE_SELECTOR.bits(),
        true,
    )
}

/// Creates the gate for the double fault handler.
fn double_fault_gate() -> GateDesc {
    GateDesc::new(
        handlers::double_fault as usize as u64,
        true,
        Some(DOUBLE_FAULT_IST_INDEX),
        0,
        KERNEL_CODE_SELECTOR.bits(),
        true,
    )
}
