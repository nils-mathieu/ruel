//! This module mainly provide constants describing the state of the GDT loaded by the kernel.

use core::alloc::Layout;
use core::mem::size_of;

use x86_64::{IstIndex, Ring, SegmentFlags, SegmentSelector, TablePtr, TaskStateSegment, VirtAddr};

use crate::cpu::paging::HHDM_OFFSET;
use crate::global::OutOfMemory;
use crate::log;
use crate::utility::BumpAllocator;

use super::paging::FOUR_KIB;

/// The selector of the kernel code segment.
pub const KERNEL_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(1, false, Ring::Zero);

/// The selector of the kernel data segment.
pub const KERNEL_DATA_SELECTOR: SegmentSelector = SegmentSelector::new(2, false, Ring::Zero);

/// The selector of the user data segment.
pub const USER_DATA_SELECTOR: SegmentSelector = SegmentSelector::new(3, false, Ring::Three);

/// The selector of the user code segment.
pub const USER_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(4, false, Ring::Three);

/// The selector of the Task State Segment.
pub const TSS_SELECTOR: SegmentSelector = SegmentSelector::new(5, false, Ring::Zero);

/// The kernel code segment value in the GDT.
pub const KERNEL_CODE_SEGMENT: u64 = SegmentFlags::ACCESSED.bits()
    | SegmentFlags::PRESENT.bits()
    | SegmentFlags::NON_SYSTEM.bits()
    | SegmentFlags::EXECUTABLE.bits()
    | SegmentFlags::READABLE.bits()
    | SegmentFlags::LONG_MODE_CODE.bits()
    | SegmentFlags::GRANULARITY_4KIB.bits()
    | SegmentFlags::MAX_LIMIT.bits();

/// The kernel data segment value in the GDT.
pub const KERNEL_DATA_SEGMENT: u64 = SegmentFlags::ACCESSED.bits()
    | SegmentFlags::PRESENT.bits()
    | SegmentFlags::NON_SYSTEM.bits()
    | SegmentFlags::WRITABLE.bits()
    | SegmentFlags::GRANULARITY_4KIB.bits()
    | SegmentFlags::SIZE_32BIT.bits()
    | SegmentFlags::MAX_LIMIT.bits();

/// The user data segment value in the GDT.
pub const USER_DATA_SEGMENT: u64 = KERNEL_DATA_SEGMENT | SegmentFlags::from_dpl(Ring::Three).bits();

/// The user code segment value in the GDT.
pub const USER_CODE_SEGMENT: u64 = KERNEL_CODE_SEGMENT | SegmentFlags::from_dpl(Ring::Three).bits();

/// The index of the double fault stack in the TSS.
pub const DOUBLE_FAULT_IST_INDEX: IstIndex = IstIndex::Index0;
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
        tss.privilege_stack_table[Ring::Zero as usize] = kernel_stack_top;
    }

    log::trace!("Initializing the GDT...");

    let gdt_phys_addr = bootstrap_allocator.allocate(Layout::new::<Gdt>())?;
    let gdt_ptr = (gdt_phys_addr as usize + HHDM_OFFSET) as *mut Gdt;

    unsafe {
        let tss_seg = x86_64::create_tss_segment(tss_ptr);
        *gdt_ptr = [
            0,
            KERNEL_CODE_SEGMENT,
            KERNEL_DATA_SEGMENT,
            USER_DATA_SEGMENT,
            USER_CODE_SEGMENT,
            tss_seg[0],
            tss_seg[1],
        ];
    }

    log::trace!("Loading the created GDT...");

    unsafe {
        let gdtr = TablePtr {
            limit: size_of::<Gdt>() as u16 - 1,
            base: gdt_ptr as *const (),
        };

        x86_64::lgdt(&gdtr);
        x86_64::ltr(TSS_SELECTOR);
        x86_64::write_cs(KERNEL_CODE_SELECTOR);
        x86_64::write_ss(KERNEL_DATA_SELECTOR);
        x86_64::write_ds(KERNEL_DATA_SELECTOR);
        x86_64::write_es(KERNEL_DATA_SELECTOR);
        x86_64::write_fs(KERNEL_DATA_SELECTOR);
        x86_64::write_gs(KERNEL_DATA_SELECTOR);
    }

    Ok(())
}
