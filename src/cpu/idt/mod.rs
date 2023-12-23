//! This module mainly provides the [`init`] function used to initialize the kernel's IDT.
//!
//! The implementation of the Interrupt Service Routines (ISRs) are located in the [`handlers`]
//! module.

use core::mem::size_of;

use x86_64::{lidt, Exception, GateDesc, Idt, Ring, TablePtr, VirtAddr};

use super::gdt::{DOUBLE_FAULT_IST_INDEX, KERNEL_CODE_SELECTOR};
use crate::cpu::idt::pic::{Irq, Irqs};
use crate::global::OutOfMemory;
use crate::log;
use crate::utility::BumpAllocator;

mod handlers;
mod pic;
pub mod pit;

/// The offset used by the PIC to remap the interrupts.
///
/// The next 16 entries in the IDT are reserved for the PIC.
const PIC_OFFSET: u8 = 32;

/// Initializes the kernel's IDT.
pub fn init(bootstrap_allocator: &mut BumpAllocator) -> Result<(), OutOfMemory> {
    let idt = bootstrap_allocator.allocate::<Idt>()?.write(Idt::EMPTY);

    log::trace!("IDT allocated at address: {:p}", idt);

    // Initilaize the IDT with our handlers.
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

    idt[PIC_OFFSET + Irq::Timer as u8] = int_gate(handlers::pic_timer as usize);
    idt[PIC_OFFSET + Irq::PS2Keyboard as u8] = int_gate(handlers::pic_ps2_keyboard as usize);
    idt[PIC_OFFSET + Irq::PS2Mouse as u8] = int_gate(handlers::pic_ps2_mouse as usize);

    match crate::io::ps2::init() {
        Ok(()) => (),
        Err(err) => {
            log::warn!("failed to initialize the PS/2 controller: {}", err);
        }
    }

    pic::init();
    pit::init();
    pic::set_irq_mask(Irqs::all().difference(Irqs::PS2_KEYBOARD | Irqs::TIMER | Irqs::PS2_MOUSE));

    log::trace!("Loading the IDT...");

    unsafe {
        lidt(&TablePtr {
            limit: size_of::<Idt>() as u16 - 1,
            base: idt as *mut _ as *const _,
        });
    }

    Ok(())
}

/// Creates a new trap gate.
fn trap_gate(handler: usize) -> GateDesc {
    GateDesc::new(handler, false, None, Ring::Zero, KERNEL_CODE_SELECTOR, true)
}

/// Creates a new interrupt gate.
fn int_gate(handler: VirtAddr) -> GateDesc {
    GateDesc::new(handler, true, None, Ring::Zero, KERNEL_CODE_SELECTOR, true)
}

/// Creates the gate for the double fault handler.
fn double_fault_gate() -> GateDesc {
    GateDesc::new(
        handlers::double_fault as VirtAddr,
        true,
        Some(DOUBLE_FAULT_IST_INDEX),
        Ring::Zero,
        KERNEL_CODE_SELECTOR,
        true,
    )
}
