//! Implementations of the ISRs for the IST of the kernel.

use ruel_sys::WakeUpPS2MouseFlags;
use x86_64::{read_cr2, InterruptStackFrame, PageFaultError};

use crate::cpu::idt::pic::Irq;
use crate::global::GlobalToken;
use crate::io::ps2::{self, PS2Status};
use crate::process::Process;

pub extern "x86-interrupt" fn division_error(_stack_frame: InterruptStackFrame) {
    panic!("Received a DIVISION_ERROR fault.");
}

pub extern "x86-interrupt" fn debug(_stack_frame: InterruptStackFrame) {
    panic!("Received a DEBUG fault/trap.");
}

pub extern "x86-interrupt" fn non_maskable_interrupt(_stack_frame: InterruptStackFrame) {
    panic!("Received a NON_MASKABLE_INTERRUPT interrupt.");
}

pub extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    panic!("Received a BREAKPOINT trap.");
}

pub extern "x86-interrupt" fn overflow(_stack_frame: InterruptStackFrame) {
    panic!("Received an OVERFLOW trap.");
}

pub extern "x86-interrupt" fn bound_range_exceeded(_stack_frame: InterruptStackFrame) {
    panic!("Received a BOUND_RANGE_EXCEEDED fault.");
}

pub extern "x86-interrupt" fn invalid_opcode(_stack_frame: InterruptStackFrame) {
    panic!("Received an INVALID_OPCODE fault.");
}

pub extern "x86-interrupt" fn device_not_available(_stack_frame: InterruptStackFrame) {
    panic!("Received a DEVICE_NOT_AVAILABLE fault.");
}

pub extern "x86-interrupt" fn double_fault(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("Received a DOUBLE_FAULT fault.");
}

pub extern "x86-interrupt" fn invalid_tss(_stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "Received an INVALID_TSS fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn segment_not_present(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "Received a SEGMENT_NOT_PRESENT fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn stack_segment_fault(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "Received a STACK_SEGMENT_FAULT fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn general_protection_fault(
    frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "\
        Received a GENERAL_PROTECTION_FAULT fault with error code {:#x}.\n\
        > RIP = {:#x}\n\
        > RSP = {:#x}\
        ",
        error_code, frame.ip, frame.sp,
    );
}

pub extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, error_code: PageFaultError) {
    panic!(
        "\
        Received a PAGE_FAULT fault.\n\
        > ERROR   = {:?}\n\
        > RIP     = {:#x}\n\
        > RSP     = {:#x}\n\
        > ADDRESS = {:#x}\
        ",
        error_code,
        frame.ip,
        frame.sp,
        read_cr2(),
    );
}

pub extern "x86-interrupt" fn x87_floating_point(_stack_frame: InterruptStackFrame) {
    panic!("Received an X87_FLOATING_POINT fault.");
}

pub extern "x86-interrupt" fn alignment_check(_stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "Received an ALIGNMENT_CHECK fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn machine_check(_stack_frame: InterruptStackFrame) -> ! {
    panic!("Received a MACHINE_CHECK fault.");
}

pub extern "x86-interrupt" fn simd_floating_point(_stack_frame: InterruptStackFrame) {
    panic!("Received an SIMD_FLOATING_POINT fault.");
}

pub extern "x86-interrupt" fn virtualization(_stack_frame: InterruptStackFrame) {
    panic!("Received a VIRTUALIZATION fault.");
}

pub extern "x86-interrupt" fn control_protection(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "Received a CONTROL_PROTECTION_EXCEPTION fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn hypervisor_injection(_stack_frame: InterruptStackFrame) {
    panic!("Received a HYPERVISOR_INJECTION_EXCEPTION fault.");
}

pub extern "x86-interrupt" fn vmm_communication(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "Received a VMM_COMMUNICATION_EXCEPTION fault with erro code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn security_exception(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "Received a SECURITY_EXCEPTION fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn pic_timer(_frame: InterruptStackFrame) {
    let glob = GlobalToken::get();
    glob.processes.for_each_mut(Process::tick);
    super::pic::end_of_interrupt(Irq::Timer);
}

pub extern "x86-interrupt" fn pic_ps2_keyboard(_frame: InterruptStackFrame) {
    let glob = GlobalToken::get();

    #[cfg(debug_assertions)]
    {
        let status = ps2::status();
        assert!(status.intersects(PS2Status::OUTPUT_BUFFER_FULL));
        assert!(!status.intersects(PS2Status::AUX_OUTPUT_BUFFER_FULL));
    }

    let scancode = ps2::read_data();

    glob.processes
        .for_each_mut(move |proc| proc.io_states.ps2_keyboard.push(scancode));

    super::pic::end_of_interrupt(Irq::PS2Keyboard);
}

pub extern "x86-interrupt" fn pic_ps2_mouse(_frame: InterruptStackFrame) {
    let glob = GlobalToken::get();

    crate::log::trace!("mouse");

    debug_assert!(ps2::status()
        .contains(PS2Status::OUTPUT_BUFFER_FULL | PS2Status::AUX_OUTPUT_BUFFER_FULL));

    let flags = ps2::read_data();
    let x_movement = ps2::read_data();
    let y_movement = ps2::read_data();

    // Check the overflow bits and discard the event if they are set.
    if flags & 0b1100_0000 != 0 {
        super::pic::end_of_interrupt(Irq::PS2Mouse);
        return;
    }

    let mut dx = u8_to_i8(x_movement);
    let mut dy = u8_to_i8(y_movement);

    if flags & 0b0001_0000 != 0 {
        dx = -dx;
    }
    if flags & 0b0010_0000 != 0 {
        dy = -dy;
    }

    let mut mouse_flags = WakeUpPS2MouseFlags::CHANGED;
    if flags & 0b0000_0001 != 0 {
        mouse_flags.insert(WakeUpPS2MouseFlags::LEFT_BUTTON);
    }
    if flags & 0b0000_0010 != 0 {
        mouse_flags.insert(WakeUpPS2MouseFlags::RIGHT_BUTTON);
    }
    if flags & 0b0000_0100 != 0 {
        mouse_flags.insert(WakeUpPS2MouseFlags::MIDDLE_BUTTON);
    }

    glob.processes.for_each_mut(move |proc| {
        proc.io_states.ps2_mouse_state = mouse_flags;
        proc.io_states.ps2_mouse_offset = [
            proc.io_states.ps2_mouse_offset[0].saturating_add(dx),
            proc.io_states.ps2_mouse_offset[1].saturating_add(dy),
        ];
    });

    super::pic::end_of_interrupt(Irq::PS2Mouse);
}

#[inline]
fn u8_to_i8(value: u8) -> i8 {
    if value > i8::MAX as u8 {
        i8::MAX
    } else {
        value as i8
    }
}
