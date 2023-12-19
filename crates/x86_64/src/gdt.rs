use core::mem::size_of;

use bitflags::bitflags;

use crate::Ring;

bitflags! {
    /// Represents a segment descriptor in the GDT.
    #[derive(Default, Debug, Clone, Copy)]
    #[repr(transparent)]
    pub struct SegmentFlags: u64 {
        /// Indicates that the segment is currently being accessed.
        const ACCESSED = 1 << 40;

        /// Indicates that the segment can be read from.
        ///
        /// This is only relevant for executable segments.
        const READABLE = 1 << 41;

        /// Indicates that the segment can be written to.
        ///
        /// This is only relevant for non-executable segments.
        const WRITABLE = 1 << 41;

        /// Indicates that the segment is executable.
        const EXECUTABLE = 1 << 43;

        /// Indicates that the segment is a data segment (as opposed to a system segment).
        const NON_SYSTEM = 1 << 44;

        /// Indicates that the segment is present in the GDT.
        const PRESENT = 1 << 47;

        /// Indicates that the segment is a long mode code segment.
        const LONG_MODE_CODE = 1 << 53;

        /// Indicates that the segment's limit is in 4KiB blocks rather than bytes.
        const GRANULARITY_4KIB = 1 << 55;

        /// Indicates that the segment is a 32-bit segment.
        const SIZE_32BIT = 1 << 54;

        /// Indicates that the segment is a system segment with an available TSS.
        ///
        /// This is only relevant for system segments.
        const AVAILABLE_TSS = 0x9 << 40;

        /// The maximum limit of a segment.
        const MAX_LIMIT = 0x000F00000000FFFF;
    }
}

impl SegmentFlags {
    /// Creates a new [`SegmentFlags`] instance from the provided limit.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if the limit is greater than
    /// `0xFFFFF`.
    #[inline]
    pub const fn from_limit(limit: u64) -> Self {
        debug_assert!(limit <= 0xFFFFF);
        Self::from_bits_retain((limit & 0xFFFF) | ((limit & 0xF0000) << 32))
    }

    /// Creates a new [`SegmentFlags`] instance from the provided privilege level.
    #[inline]
    pub const fn from_dpl(ring: Ring) -> Self {
        Self::from_bits_retain((ring as u64) << 45)
    }

    /// Creates a new [`SegmentFlags`] instance from the provided base address.
    #[inline]
    pub const fn from_base(base: u32) -> Self {
        let base = base as u64;
        Self::from_bits_retain((base & 0xFFFFF) << 16 | (base & 0xFF000000) << 32)
    }
}

/// Represents a segment selector.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    /// Creates a new segment selector.
    ///
    /// # Arguments
    ///
    /// - `index` - The index of the segment in the GDT. In debug mode, the function panics if
    ///   it is greater than 8192.
    ///
    /// - `local` - Whether the segment is part of the current LDT instead of the GDT.
    ///
    /// - `ring` - The privilege level of the segment.
    #[inline]
    pub const fn new(index: usize, local: bool, privilege: Ring) -> Self {
        debug_assert!(index < 8192);
        Self((index as u16) << 3 | (local as u16) << 2 | privilege as u16)
    }

    /// Returns the index of the segment in the GDT or LDT.
    #[inline]
    pub const fn index(self) -> usize {
        (self.0 >> 3) as usize
    }

    /// Returns whether the segment is part of the current LDT instead of the GDT.
    #[inline]
    pub const fn is_local(self) -> bool {
        self.0 & (1 << 2) != 0
    }

    /// Returns whether the segment is part of the GDT instead of the current LDT.
    #[inline]
    pub const fn is_global(self) -> bool {
        self.0 & (1 << 2) == 0
    }

    /// Returns the privilege level of the segment.
    #[inline]
    pub const fn privilege(self) -> Ring {
        unsafe { Ring::from_raw(self.0 as u8 & 0b11) }
    }

    /// Returns the raw value of the segment selector.
    #[inline]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Creates a new [`SegmentSelector`] from the provided raw value.
    #[inline]
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

/// The content of a Task State Segment.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    pub reserved0: u32,

    /// The privilege stack table, responsible for storing the stack pointers
    /// that should be used when switching to the corresponding privilege level.
    pub privilege_stack_table: [usize; 3],
    pub reserved1: u64,

    /// The interrupt stack table.
    ///
    /// When defining a gate descriptor to register an interrupt service routine, it is
    /// possible to specify a stack index. This is the index that will be used to select
    /// the stack pointer to use serving the interrupt or trap.
    pub interrupt_stack_table: [usize; 7],

    pub reserved2: u64,
    pub reserved3: u16,

    /// The base address of the I/O permission bitmap.
    pub iomap_base: u16,
}

/// A possible IST index (within the [`TaskStateSegment`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum IstIndex {
    Index0,
    Index1,
    Index2,
    Index3,
    Index4,
    Index5,
    Index6,
}

/// Creates a system segment descriptor for the provided task state segment.
pub fn create_tss_segment(tss: *const TaskStateSegment) -> [u64; 2] {
    const LIMIT: u64 = size_of::<TaskStateSegment>() as u64 - 1;
    const FLAGS: SegmentFlags = SegmentFlags::PRESENT
        .union(SegmentFlags::AVAILABLE_TSS)
        .union(SegmentFlags::from_limit(LIMIT));

    let base = tss as usize as u64;

    [
        (FLAGS | SegmentFlags::from_base(base as u32)).bits(),
        base >> 32,
    ]
}
