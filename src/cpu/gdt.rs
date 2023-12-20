//! This module mainly provide constants describing the state of the GDT loaded by the kernel.

use core::mem::size_of;

use x86_64::{IstIndex, Ring, SegmentFlags, SegmentSelector, TablePtr, TaskStateSegment, VirtAddr};

use super::paging::{HhdmToken, FOUR_KIB};
use crate::global::OutOfMemory;
use crate::log;
use crate::utility::BumpAllocator;

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
pub fn init(
    bootstrap_allocator: &mut BumpAllocator,
    kernel_stack_top: VirtAddr,
    hhdm: HhdmToken,
) -> Result<(), OutOfMemory> {
    let double_fault_stack =
        bootstrap_allocator.allocate_slice::<u8>(hhdm, DOUBLE_FAULT_STACK_SIZE)?;
    let double_fault_stack = double_fault_stack.as_ptr() as VirtAddr + double_fault_stack.len();

    log::trace!(
        "Double fault stack allocated at address: {:#x}",
        double_fault_stack,
    );

    let tss = bootstrap_allocator
        .allocate::<TaskStateSegment>(hhdm)?
        .write(TaskStateSegment::EMPTY);

    log::trace!("TSS allocated at address: {:p}", tss);

    tss.set_ist(DOUBLE_FAULT_IST_INDEX, double_fault_stack);
    tss.set_privilege_stack(Ring::Zero, kernel_stack_top);

    let gdt = bootstrap_allocator.allocate::<Gdt>(hhdm)?;

    log::trace!("GDT allocated at address: {:p}", gdt);

    let tss_seg = x86_64::create_tss_segment(tss);
    gdt.write([
        0,
        KERNEL_CODE_SEGMENT,
        KERNEL_DATA_SEGMENT,
        USER_DATA_SEGMENT,
        USER_CODE_SEGMENT,
        tss_seg[0],
        tss_seg[1],
    ]);

    log::trace!("Loading the created GDT...");

    unsafe {
        let gdtr = TablePtr {
            limit: size_of::<Gdt>() as u16 - 1,
            base: gdt as *mut _ as *const (),
        };

        x86_64::lgdt(&gdtr);
        x86_64::write_cs(KERNEL_CODE_SELECTOR);
        x86_64::write_ss(KERNEL_DATA_SELECTOR);
        x86_64::write_ds(KERNEL_DATA_SELECTOR);
        x86_64::write_es(KERNEL_DATA_SELECTOR);
        x86_64::write_fs(KERNEL_DATA_SELECTOR);
        x86_64::write_gs(KERNEL_DATA_SELECTOR);
        x86_64::ltr(TSS_SELECTOR);
    }

    Ok(())
}
