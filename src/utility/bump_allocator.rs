use core::alloc::Layout;
use core::mem::MaybeUninit;

use x86_64::PhysAddr;

use crate::cpu::paging::{HhdmToken, HHDM_OFFSET};
use crate::global::OutOfMemory;

/// A memory allocator that uses a pointer bumping strategy to allocate new memory pages.
///
/// This has the advantage of being very simple and fast, but the main disadvantage is that
/// memory cannot be freed easily. This allocator should generally only be used to allocate
/// static structures for the kernel during the boot process.
///
/// # Representation Invariants
///
/// ```txt
/// Memory:
/// +----------------------------------+
/// | |              |                 |
/// +----------------------------------+
///   ^              ^
///   top            base
/// ```
///
/// The bump allocator allocates memory from the top of the memory region it owns. `base` remains
/// unchanged during the lifetime of the bump allocator, while `top` is decremented every time
/// a new page is allocated.
pub struct BumpAllocator {
    base: PhysAddr,
    top: PhysAddr,

    original_top: PhysAddr,
}

impl BumpAllocator {
    /// Creates a new [`BumpAllocator`].
    ///
    /// # Arguments
    ///
    /// - `base`: The first byte of the memory region the allocator owns.
    ///
    /// - `top`: The first byte of the memory region the allocator does *not* own.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the memory region `[base, top)` is not used by anything else.
    /// The created [`BumpAllocator`] instance logically "takes ownership" of the memory region
    /// `[base, top)`.
    ///
    /// # Panics
    ///
    /// This function panics if `base` is greater than `top`.
    #[inline]
    pub unsafe fn new(base: PhysAddr, top: PhysAddr) -> Self {
        assert!(
            base <= top,
            "attempted to create a BumpAllocator with base > top",
        );

        Self {
            base,
            top,
            original_top: top,
        }
    }

    /// Returns the current top of the bump allocator.
    ///
    /// This is the moving pointer that points to the next free byte in the memory region. Note
    /// that this does not necessarily mean that the next byte will be used for an allocation,
    /// as it might not be properly aligned.
    #[inline]
    pub fn top(&self) -> PhysAddr {
        self.top
    }

    /// Returns the original top of the bump allocator.
    #[inline]
    pub fn original_top(&self) -> u64 {
        self.original_top
    }

    /// Allocates memory for the provided layout.
    ///
    /// The returned physical address is guaranteed to be aligned to `layout.align()`, and to
    /// be at least `layout.size()` bytes large.
    pub fn allocate_phys(&mut self, layout: Layout) -> Result<PhysAddr, OutOfMemory> {
        let size = layout.size() as u64;
        let align = layout.align() as u64;

        let mut ret = self.top;

        ret = ret.checked_sub(size).ok_or(OutOfMemory)?;
        ret &= !(align - 1);

        if ret < self.base {
            return Err(OutOfMemory);
        }

        self.top = ret;
        Ok(ret)
    }

    /// Allocates an isntance of `T` without initializing it.
    pub fn allocate<T>(
        &mut self,
        _hhdm: HhdmToken,
    ) -> Result<&'static mut MaybeUninit<T>, OutOfMemory> {
        let layout = Layout::new::<T>();
        let phys_addr = self.allocate_phys(layout)?;
        let virt_addr = (phys_addr as usize + HHDM_OFFSET) as *mut MaybeUninit<T>;
        debug_assert!(virt_addr as usize & (layout.align() - 1) == 0);
        Ok(unsafe { &mut *virt_addr })
    }

    /// Allocates a slice of `T`s without initializing it.
    pub fn allocate_slice<T>(
        &mut self,
        _hhdm: HhdmToken,
        size: usize,
    ) -> Result<&'static mut [T], OutOfMemory> {
        let layout = Layout::array::<T>(size).map_err(|_| OutOfMemory)?;
        let phys_addr = self.allocate_phys(layout)?;
        let virt_addr = (phys_addr as usize + HHDM_OFFSET) as *mut T;
        debug_assert!(virt_addr as usize & (layout.align() - 1) == 0);
        Ok(unsafe { core::slice::from_raw_parts_mut(virt_addr, size) })
    }
}
