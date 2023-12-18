//! Defines the raw structures of the Limine boot protocol.

use core::mem::transmute;

use crate::utility::Volatile;

/// Represents a pointer that is written to by the bootloader while loading the kernel. We need to
/// use [`Volatile`] semantics to prevent the compiler from optimizing away the reads of the
/// pointer (from its perspective, the pointer is never written to, and thus always null).
#[derive(Debug)]
pub struct ResponsePtr<T: ?Sized>(Volatile<*const T>);

impl<T> ResponsePtr<T> {
    /// A null [`ResponsePtr<T>`].
    #[allow(clippy::declare_interior_mutable_const)]
    pub const NULL: Self = Self(Volatile::new(core::ptr::null()));
}

impl<T: ?Sized> ResponsePtr<T> {
    /// Returns a reference to the pointed value, if the pointer is not null.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is actually valid.
    ///
    /// # Remarks
    ///
    /// If the [`ResponsePtr`] is only written to by the bootloader (and that the bootloader
    /// is actually compliant with the standard), then this function is safe to call as long
    /// as the bootloader-reclaimable memory region is not overwritten.
    #[inline]
    pub unsafe fn read<'a>(&self) -> Option<&'a T> {
        unsafe { transmute::<*const T, Option<&T>>(*self.0) }
    }
}

// SAFETY:
//  `ResponsePtr` is not necessarily a unique poiner to the value it points to (though in the
//  context of the Limine boot protocol, that may be the case). If `T` is `Sync`, then it is safe
//  to send a `ResponsePtr<T>` to another thread.
unsafe impl<T: Sync> Send for ResponsePtr<T> {}
unsafe impl<T: Sync> Sync for ResponsePtr<T> {}

impl<T: ?Sized> Clone for ResponsePtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// The base version of the protocol implemented by the kernel.
pub const BASE_VERSION: u64 = 1;

/// This static variable is read by the bootloader to determine the base revision the kernel
/// knows about.
///
/// If the bootloader is able to understand the kernel's revision, it will set the last component
/// of this array to 0. Otherwise, it will leave it unchanged.
#[used(linker)]
static BASE_REVISION: Volatile<[u64; 3]> =
    Volatile::new([0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, BASE_VERSION]);

/// Returns whether the bootloader supports the base revision expected by the kernel.
#[inline]
pub fn base_revision_supported() -> bool {
    BASE_REVISION[2] == 0
}

/// A request identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Id(pub [u64; 4]);

impl Id {
    /// Create a common ID from the provided last two components.
    ///
    /// # Remarks
    ///
    /// All Limine requests have the same first two components. This function simply creates a new
    /// [`Id`] using those common components and the provided last two components.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ruel::boot::limine::raw::Id;
    /// assert_eq!(Id::common(1, 2), Id([0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, 1, 2]));
    /// ```
    #[inline]
    pub const fn common(c: u64, d: u64) -> Self {
        Self([0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, c, d])
    }
}

/// The type used to represent revision numbers.
pub type Revision = u64;

/// The request ID for [`MemoryMapRequest`].
pub const ENTRY_POINT_REQUEST: Id = Id::common(0x13d86c035a1cd3e1, 0x2b0caa89d8f3026a);

/// The signature of the entry point function expected by the bootloader.
pub type EntryPoint = unsafe extern "C" fn() -> !;

/// <https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#entry-point-feature>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EntryPointRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<EntryPointResponse>,
    pub entry: EntryPoint,
}

/// The response type associated with [`EntryPointRequest`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EntryPointResponse {
    pub revision: Revision,
}
