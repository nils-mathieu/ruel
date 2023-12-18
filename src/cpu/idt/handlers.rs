//! Implementations of the ISRs for the IST of the kernel.

use super::raw::InterruptStackFrame;

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
    error_code: u64,
) -> ! {
    panic!(
        "Received a DOUBLE_FAULT fault with error code {:#x}.",
        error_code
    );
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
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "Received a GENERAL_PROTECTION_FAULT fault with error code {:#x}.",
        error_code
    );
}

pub extern "x86-interrupt" fn page_fault(_stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "Received a PAGE_FAULT fault with error code {:#x}.",
        error_code
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
