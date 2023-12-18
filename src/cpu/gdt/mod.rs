use core::alloc::Layout;
use core::arch::asm;
use core::mem::size_of;

use crate::cpu::paging::HHDM_OFFSET;
use crate::log;
use crate::mem::{BumpAllocator, OutOfMemory, VirtAddr};

mod raw;
use self::raw::*;

use super::paging::FOUR_KIB;

/// The selector of the kernel code segment.
pub const KERNEL_CODE_SELECTOR: u16 = make_selector(1, false, 0);

/// The selector of the kernel data segment.
pub const KERNEL_DATA_SELECTOR: u16 = make_selector(2, false, 0);

/// The selector of the user data segment.
pub const USER_DATA_SELECTOR: u16 = make_selector(3, false, 3);

/// The selector of the user code segment.
pub const USER_CODE_SELECTOR: u16 = make_selector(4, false, 3);

/// The selector of the Task State Segment.
pub const TSS_SELECTOR: u16 = make_selector(5, false, 0);

/// The kernel code segment value in the GDT.
pub const KERNEL_CODE_SEGMENT: u64 = SEGMENT_ACCESSED
    | SEGMENT_PRESENT
    | SEGMENT_DATA
    | SEGMENT_EXECUTABLE
    | SEGMENT_READABLE
    | SEGMENT_LONG_MODE_CODE
    | SEGMENT_GRANULARITY_4KIB
    | SEGMENT_MAX_LIMIT;

/// The kernel data segment value in the GDT.
pub const KERNEL_DATA_SEGMENT: u64 = SEGMENT_ACCESSED
    | SEGMENT_PRESENT
    | SEGMENT_DATA
    | SEGMENT_WRITABLE
    | SEGMENT_GRANULARITY_4KIB
    | SEGMENT_SIZE_32BIT
    | SEGMENT_MAX_LIMIT;

/// The user data segment value in the GDT.
pub const USER_DATA_SEGMENT: u64 = KERNEL_DATA_SEGMENT | SEGMENT_USER;

/// The user code segment value in the GDT.
pub const USER_CODE_SEGMENT: u64 = KERNEL_CODE_SEGMENT | SEGMENT_USER;

/// The index of the double fault stack in the TSS.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
/// The size of the double fault stack.
pub const DOUBLE_FAULT_STACK_SIZE: usize = FOUR_KIB * 8;

/// The type responsible for holding the GDT in its entirety.
type Gdt = [u64; 7];

/// Initializes our own GDT.
///
/// # Safety
///
/// This function assumes that the global HHDM has been initialized.
///
/// The provided `kernel_stack_top` must be the top of the kernel stack.
pub unsafe fn init(
    bootstrap_allocator: &mut BumpAllocator,
    kernel_stack_top: VirtAddr,
) -> Result<(), OutOfMemory> {
    log::trace!("Initializing the TSS...");

    let double_fault_stack_phys_addr =
        bootstrap_allocator.allocate(Layout::new::<[u8; DOUBLE_FAULT_STACK_SIZE]>())?;
    let double_fault_stack =
        double_fault_stack_phys_addr as usize + HHDM_OFFSET + DOUBLE_FAULT_STACK_SIZE;

    let tss_phys_addr = bootstrap_allocator.allocate(Layout::new::<TaskStateSegment>())?;
    let tss_ptr = (tss_phys_addr as usize + HHDM_OFFSET) as *mut TaskStateSegment;

    unsafe {
        core::ptr::write_bytes(tss_ptr, 0x00, 1);

        let tss = &mut *tss_ptr;

        tss.iomap_base = size_of::<TaskStateSegment>() as u16;
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = double_fault_stack;
        tss.privilege_stack_table[0] = kernel_stack_top;
    }

    log::trace!("Initializing the GDT...");

    let gdt_phys_addr = bootstrap_allocator.allocate(Layout::new::<Gdt>())?;
    let gdt_ptr = (gdt_phys_addr as usize + HHDM_OFFSET) as *mut Gdt;

    unsafe {
        let gdt = &mut *gdt_ptr;

        gdt[0] = 0; // null descriptor
        gdt[1] = KERNEL_CODE_SEGMENT;
        gdt[2] = KERNEL_DATA_SEGMENT;
        gdt[3] = USER_DATA_SEGMENT;
        gdt[4] = USER_CODE_SEGMENT;
        [gdt[5], gdt[6]] = make_tss_segment(tss_ptr);
    }

    log::trace!("Loading the created GDT...");

    #[repr(C, packed)]
    struct Gdtr {
        limit: u16,
        base: usize,
    }

    let gdtr = Gdtr {
        limit: size_of::<Gdt>() as u16 - 1,
        base: gdt_ptr as usize,
    };

    unsafe {
        // Load the GDT using the `lgdt` instruction.
        asm!(
            "lgdt [{}]",
            in(reg) &gdtr,
            options(nostack, readonly, preserves_flags)
        );

        // Load the TSS using the `ltr` instruction.
        asm!(
            "ltr {:x}",
            in(reg) TSS_SELECTOR,
            options(nostack, nomem, preserves_flags)
        );

        // Update the segment registers.
        asm!(
            "
            mov ss, {0:x}
            mov ds, {0:x}
            mov es, {0:x}
            mov fs, {0:x}
            mov gs, {0:x}
            ",
            in(reg) KERNEL_DATA_SELECTOR,
            options(nostack, nomem, preserves_flags)
        );

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
            const KERNEL_CODE_SELECTOR as usize, // this is important, the selector is 16-bit wide, but we need to push a full word on the stack.
            tmp = lateout(reg) _,
            options(preserves_flags, nomem),
        );
    }

    Ok(())
}
