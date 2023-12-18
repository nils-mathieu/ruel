use core::mem::size_of;

/// Creates a new segment selector from the provided index, local/global bit, and privilege level.
pub const fn make_selector(index: u16, local: bool, ring: u16) -> u16 {
    assert!(ring <= 3);
    index << 3 | (local as u16) << 2 | ring
}

/// Indicates that the segment has been accessed.
pub const SEGMENT_ACCESSED: u64 = 1 << 40;
/// Indicates that the segment can be read from.
///
/// This is only relevant for executable segments.
pub const SEGMENT_READABLE: u64 = 1 << 41;
/// Indicates that the segment can be executed.
///
/// This is only relevant for non-executable segments.
pub const SEGMENT_WRITABLE: u64 = 1 << 41;
/// Indicates that the segment is executable.
pub const SEGMENT_EXECUTABLE: u64 = 1 << 43;
/// Indicates that the segment is a data segment (as opposed to a system segment).
pub const SEGMENT_DATA: u64 = 1 << 44;
/// Whether the segment is present.
pub const SEGMENT_PRESENT: u64 = 1 << 47;
/// Whether the segment can be accessed from ring 3.
pub const SEGMENT_USER: u64 = 1 << 45;
/// Indicates that the segment is a long mode code segment.
pub const SEGMENT_LONG_MODE_CODE: u64 = 1 << 53;
/// Indicaets that the segment's limit is in 4KiB blocks rather than bytes.
pub const SEGMENT_GRANULARITY_4KIB: u64 = 1 << 55;
/// The maximum limit of a segment.
pub const SEGMENT_MAX_LIMIT: u64 = 0x000F00000000FFFF;
/// Indicates that the segment is a 32-bit segment.
pub const SEGMENT_SIZE_32BIT: u64 = 1 << 54;
/// Indicates that the segment represents an available TSS.
///
/// Only relevant for system segments.
pub const SEGMENT_AVAILABLE_TSS: u64 = 0x9 << 40;

/// The content of a Task State Segment.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    pub reserved0: u32,
    pub privilege_stack_table: [usize; 3],
    pub reserved1: u64,
    pub interrupt_stack_table: [usize; 7],
    pub reserved2: u64,
    pub reserved3: u16,
    pub iomap_base: u16,
}

/// Creates a pair of GDT entries for the provided TSS.
pub fn make_tss_segment(tss: *const TaskStateSegment) -> [u64; 2] {
    let addr = tss as usize as u64;
    let limit = size_of::<TaskStateSegment>() as u64 - 1;

    let low = SEGMENT_PRESENT
        | SEGMENT_AVAILABLE_TSS
        | limit
        | (addr & 0xFFFFFF) << 16
        | (addr & 0xFF000000) << 32;
    let high = addr >> 32;

    [low, high]
}
