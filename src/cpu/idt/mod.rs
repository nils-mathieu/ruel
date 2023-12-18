//! This module mainly provides the [`init`] function used to initialize the kernel's IDT.
//!
//! The implementation of the Interrupt Service Routines (ISRs) are located in the [`handlers`]
//! module.

use core::alloc::Layout;
use core::arch::asm;
use core::mem::size_of;

use crate::log;
use crate::mem::{BumpAllocator, OutOfMemory};

mod handlers;
mod raw;

use self::raw::*;

use super::gdt::KERNEL_CODE_SELECTOR;
use super::paging::HHDM_OFFSET;

/// Initializes the kernel's IDT.
pub fn init(bootstrap_allocator: &mut BumpAllocator) -> Result<(), OutOfMemory> {
    log::trace!("Initializing the IDT...");

    let idt_phys_addr = bootstrap_allocator.allocate(Layout::new::<Idt>())?;
    let idt = unsafe { &mut *((idt_phys_addr as usize + HHDM_OFFSET) as *mut Idt) };

    idt[VECNBR_DIVISION_ERROR] = trap_gate(handlers::division_error as usize);
    idt[VECNBR_DEBUG] = trap_gate(handlers::debug as usize);
    idt[VECNBR_NON_MASKABLE_INTERRUPT] = interrupt_gate(handlers::non_maskable_interrupt as usize);
    idt[VECNBR_BREAKPOINT] = trap_gate(handlers::breakpoint_handler as usize);
    idt[VECNBR_OVERFLOW] = trap_gate(handlers::overflow as usize);
    idt[VECNBR_BOUND_RANGE_EXCEEDED] = trap_gate(handlers::bound_range_exceeded as usize);
    idt[VECNBR_INVALID_OPCODE] = trap_gate(handlers::invalid_opcode as usize);
    idt[VECNBR_DEVICE_NOT_AVAILABLE] = trap_gate(handlers::device_not_available as usize);
    idt[VECNBR_DOUBLE_FAULT] = double_fault_gate();
    idt[VECNBR_INVALID_TSS] = trap_gate(handlers::invalid_tss as usize);
    idt[VECNBR_SEGMENT_NOT_PRESENT] = trap_gate(handlers::segment_not_present as usize);
    idt[VECNBR_STACK_SEGMENT_FAULT] = trap_gate(handlers::stack_segment_fault as usize);
    idt[VECNBR_GENERAL_PROTECTION_FAULT] = trap_gate(handlers::general_protection_fault as usize);
    idt[VECNBR_PAGE_FAULT] = trap_gate(handlers::page_fault as usize);
    idt[VECNBR_X87_FLOATING_POINT_EXCEPTION] = trap_gate(handlers::x87_floating_point as usize);
    idt[VECNBR_ALIGNMENT_CHECK] = trap_gate(handlers::alignment_check as usize);
    idt[VECNBR_MACHINE_CHECK] = trap_gate(handlers::machine_check as usize);
    idt[VECNBR_SIMD_FLOATING_POINT_EXCEPTION] = trap_gate(handlers::simd_floating_point as usize);
    idt[VECNBR_VIRTUALIZATION_EXCEPTION] = trap_gate(handlers::virtualization as usize);
    idt[VECNBR_CONTROL_PROTECTION_EXCEPTION] = trap_gate(handlers::control_protection as usize);
    idt[VECNBR_HYPERVISOR_INJECTION_EXCEPTION] = trap_gate(handlers::hypervisor_injection as usize);
    idt[VECNBR_VMM_COMMUNICATION_EXCEPTION] = trap_gate(handlers::vmm_communication as usize);
    idt[VECNBR_SECURITY_EXCEPTION] = trap_gate(handlers::security_exception as usize);

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
    GateDesc::new(handler as u64, false, 0, 0, KERNEL_CODE_SELECTOR, true)
}

/// Creates a new interrupt gate.
fn interrupt_gate(handler: usize) -> GateDesc {
    GateDesc::new(handler as u64, true, 0, 0, KERNEL_CODE_SELECTOR, true)
}

/// Creates the gate for the double fault handler.
fn double_fault_gate() -> GateDesc {
    GateDesc::new(
        handlers::double_fault as usize as u64,
        true,
        1,
        0,
        KERNEL_CODE_SELECTOR,
        true,
    )
}
