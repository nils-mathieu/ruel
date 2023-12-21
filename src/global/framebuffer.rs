use core::cell::UnsafeCell;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use ruel_sys::{Framebuffer, ProcessId};

use crate::utility::array_vec::ArrayVec;

/// Manages the framebuffers available to the kernel.
pub struct Framebuffers {
    /// Metadata about acquired framebuffers.
    metadata: UnsafeCell<[FramebufferMeta; 4]>,
    /// The owner of the framebuffers.
    ///
    /// The special value `ProcessId::MAX` indicates that no process currently owns the
    /// framebuffers.
    owner: AtomicUsize,
    /// The list of framebuffers available to the kernel.
    framebuffers: ArrayVec<Framebuffer, 4>,
}

unsafe impl Sync for Framebuffers {}
unsafe impl Send for Framebuffers {}

impl Framebuffers {
    /// Creates a new [`Framebuffers`] instance.
    pub fn new(framebuffers: ArrayVec<Framebuffer, 4>) -> Self {
        Self {
            owner: AtomicUsize::new(ProcessId::MAX),
            framebuffers,
            metadata: UnsafeCell::new([FramebufferMeta::default(); 4]),
        }
    }

    /// Attempts to acquire the framebuffers for the given process.
    ///
    /// If the framebuffers are already owned by the current process, this function returns `true`
    /// regardless.
    #[inline]
    pub fn acquire(&self, id: ProcessId) -> bool {
        match self
            .owner
            .compare_exchange(ProcessId::MAX, id, Acquire, Relaxed)
        {
            Ok(_) => true,
            Err(current) => current == id,
        }
    }

    /// Releases the framebuffers from the given process.
    #[inline]
    pub fn release(&self, id: ProcessId) -> bool {
        self.owner
            .compare_exchange(id, ProcessId::MAX, Release, Relaxed)
            .is_ok()
    }

    /// Returns the framebuffers available to the kernel.
    #[inline]
    pub fn as_slice(&self) -> &[Framebuffer] {
        &self.framebuffers
    }

    /// Returns the metadata.
    ///
    ///
    /// # Safety
    ///
    /// The context calling the function must has acquired the framebuffers.
    ///
    /// Also, this function must not be used to create multiple mutable references to the
    /// metadata.
    #[inline]
    #[allow(clippy::mut_from_ref)] // function is unsafe to call for this reason
    pub unsafe fn metadata_mut(&self) -> &mut [FramebufferMeta] {
        unsafe { &mut *self.metadata.get() }
    }
}

/// Stores metadata about an aquired framebuffer.
#[derive(Clone, Copy, Debug, Default)]
pub struct FramebufferMeta {
    /// The virtual address where the framebuffer is mapped in the process's address space.
    ///
    /// If `0`, the framebuffer is not mapped even though the process has acquired it.
    pub virt_address: usize,
    /// The size of the mapping in the process's address space.
    pub virt_size: usize,
}
