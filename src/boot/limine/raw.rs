//! Defines the raw structures of the Limine boot protocol.

use core::ffi::{c_char, CStr};

use crate::utility::Volatile;

/// A pointer into the bootloader-reclaimable memory of the kernel.
#[repr(transparent)]
pub struct LiminePtr<T: ?Sized>(*const T);

impl<T> LiminePtr<T> {
    /// A null [`LiminePtr<T>`].
    pub const NULL: Self = Self(core::ptr::null());

    /// Creates a slice from this pointer.
    ///
    /// # Safety
    ///
    /// The memory pointed to by this pointer must be valid the lifetime of the created slice.
    #[inline]
    pub unsafe fn slice<'a>(self, len: usize) -> &'a [T] {
        unsafe { core::slice::from_raw_parts(self.0, len) }
    }
}

impl<T: ?Sized> LiminePtr<T> {
    /// Returns whether the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Turns this pointer into a regular Rust reference.
    ///
    /// # Safety
    ///
    /// The memory pointed to by this pointer must be valid.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> &'a T {
        unsafe { &*self.0 }
    }

    /// Casts this pointer to a pointer to a different type.
    #[inline]
    pub const fn cast<U>(self) -> LiminePtr<U> {
        LiminePtr(self.0 as *const U)
    }
}

impl LiminePtr<c_char> {
    /// Turns this pointer into a C string.
    ///
    /// # Safety
    ///
    /// The memory pointed to by this pointer must be valid and null-terminated.
    #[inline]
    pub unsafe fn as_cstr<'a>(self) -> &'a CStr {
        unsafe { CStr::from_ptr(self.0) }
    }
}

impl<T: ?Sized> core::fmt::Debug for LiminePtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: ?Sized> Clone for LiminePtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for LiminePtr<T> {}

// SAFETY:
//  `LiminePtr` is not necessarily a unique poiner to the value it points to (though in the
//  context of the Limine boot protocol, that may be the case). If `T` is `Sync`, then it is safe
//  to send a `ResponsePtr<T>` to another thread.
unsafe impl<T: ?Sized + Sync> Send for LiminePtr<T> {}
unsafe impl<T: ?Sized + Sync> Sync for LiminePtr<T> {}

/// Represents a pointer that is written to by the bootloader while loading the kernel. We need to
/// use [`Volatile`] semantics to prevent the compiler from optimizing away the reads of the
/// pointer (from its perspective, the pointer is never written to, and thus always null).
#[derive(Debug)]
pub struct ResponsePtr<T: ?Sized>(Volatile<LiminePtr<T>>);

impl<T> ResponsePtr<T> {
    /// A null [`ResponsePtr<T>`].
    #[allow(clippy::declare_interior_mutable_const)]
    pub const NULL: Self = Self(Volatile::new(LiminePtr::NULL));
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
        if self.0.is_null() {
            None
        } else {
            Some(unsafe { (*self.0).as_ref() })
        }
    }
}

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

/// The requset ID for [`BootloaderInfoRequest`].
pub const BOOTLOADER_INFO_REQUEST: Id = Id::common(0xf55038d8e2a1202f, 0x279426fcf5f59740);

/// <https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#bootloader-info-feature>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BootloaderInfoRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<BootloaderInfoResponse>,
}

/// The response type associated with [`BootloaderInfoRequest`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BootloaderInfoResponse {
    pub revision: Revision,
    pub name: LiminePtr<c_char>,
    pub version: LiminePtr<c_char>,
}

/// The request ID for [`EntryPointRequest`].
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

/// The request ID for [`MemmapRequest`].
pub const MEMMAP_REQUEST: Id = Id::common(0x67cf3d9d378a806f, 0xe304acdfc50c3c62);

/// <https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#memory-map-feature>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MemmapRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<MemmapResponse>,
}

/// The response type associated with [`MemmapRequest`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MemmapResponse {
    pub revision: Revision,
    pub entry_count: u64,
    pub entries: LiminePtr<LiminePtr<MemmapEntry>>,
}

/// An entry in the memory map, as reported by the bootloader.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MemmapEntry {
    pub base: u64,
    pub length: u64,
    pub ty: MemmapType,
}

/// The type of a memory map entry.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemmapType(pub u32);

impl MemmapType {
    pub const USABLE: Self = Self(0);
    pub const RESERVED: Self = Self(1);
    pub const ACPI_RECLAIMABLE: Self = Self(2);
    pub const ACPI_NVS: Self = Self(3);
    pub const BAD_MEMORY: Self = Self(4);
    pub const BOOTLOADER_RECLAIMABLE: Self = Self(5);
    pub const KERNEL_AND_MODULES: Self = Self(6);
    pub const FRAMEBUFFER: Self = Self(7);
}

impl core::fmt::Debug for MemmapType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::USABLE => write!(f, "USABLE"),
            Self::RESERVED => write!(f, "RESERVED"),
            Self::ACPI_RECLAIMABLE => write!(f, "ACPI_RECLAIMABLE"),
            Self::ACPI_NVS => write!(f, "ACPI_NVS"),
            Self::BAD_MEMORY => write!(f, "BAD_MEMORY"),
            Self::BOOTLOADER_RECLAIMABLE => write!(f, "BOOTLOADER_RECLAIMABLE"),
            Self::KERNEL_AND_MODULES => write!(f, "KERNEL_AND_MODULES"),
            Self::FRAMEBUFFER => write!(f, "FRAMEBUFFER"),
            _ => f.debug_tuple("MemmapType").field(&self.0).finish(),
        }
    }
}

/// The request ID for [`HhdmRequest`].
pub const HHDM_REQUEST: Id = Id::common(0x48dcf1cb8ad2b852, 0x63984e959a98244b);

/// <https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#hhdm-higher-half-direct-map-feature>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct HhdmRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<HhdmResponse>,
}

/// The response type associated with [`HhdmRequest`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct HhdmResponse {
    pub revision: Revision,
    pub offset: u64,
}

/// The request ID for [`KernelAddressRequest`].
pub const KERNEL_ADDRESS_REQUEST: Id = Id::common(0x71ba76863cc55f63, 0xb2644a48c516a487);

/// <https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#kernel-address-feature>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct KernelAddressRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<KernelAddressResponse>,
}

/// The response type associated with [`KernelAddressRequest`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct KernelAddressResponse {
    pub revision: Revision,
    pub physical_base: u64,
    pub virtual_base: u64,
}

/// The request ID for [`ModuleRequest`].
pub const MODULE_REQUEST: Id = Id::common(0x3e7e279702be32af, 0xca1c4f3bd1280cee);

pub struct InternalModule {
    pub path: LiminePtr<c_char>,
    pub cmdline: LiminePtr<c_char>,
    pub flags: u64,
}

/// <https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#module-feature>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ModuleRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<ModuleResponse>,

    // Request revision 1
    pub internal_module_count: u64,
    pub internal_modules: LiminePtr<LiminePtr<InternalModule>>,
}

/// The response type associated with [`ModuleRequest`].
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ModuleResponse {
    pub revision: Revision,
    pub module_count: u64,
    pub modules: LiminePtr<LiminePtr<File>>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Uuid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct File {
    pub revision: u64,
    pub address: LiminePtr<u8>,
    pub size: u64,
    pub path: LiminePtr<c_char>,
    pub cmdline: LiminePtr<c_char>,
    pub media_type: u64,
    pub unused: u32,
    pub tftp_ip: u32,
    pub tftp_port: u32,
    pub partition_index: u32,
    pub mbr_disk_id: u32,
    pub gpt_disk_uuid: Uuid,
    pub gpt_part_uuid: Uuid,
    pub part_uuid: Uuid,
}

pub const MEDIA_TYPE_GENERIC: u64 = 0;
pub const MEDIA_TYPE_OPTICAL: u64 = 1;
pub const MEDIA_TYPE_TFTP: u64 = 2;
