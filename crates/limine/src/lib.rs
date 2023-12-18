//! This crate provides an implementation of the [Limine Boot Protocol](PROTOCOL).
//!
//! [PROTOCOL]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md

#![no_std]
#![feature(used_with_arg)]

use core::ffi::c_char;

mod limine_ptr;
use bitflags::bitflags;
pub use limine_ptr::*;

macro_rules! create_loose_enum {
    (
        $(#[$($attr:meta)*])*
        $vis:vis struct $name:ident: $inner:ty {
            $(
                $(#[$($variant_attr:meta)*])*
                const $variant:ident = $value:expr;
            )*
        }
    ) => {
        $(#[$($attr)*])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name($inner);

        impl $name {
            $(
                $(#[$($variant_attr)*])*
                pub const $variant: Self = Self($value);
            )*

            #[doc = ::core::concat!("Creates a new [`", stringify!($name), "`] from the provided raw value.")]
            #[inline]
            pub fn from_raw(raw: $inner) -> Self {
                Self(raw)
            }

            #[doc = ::core::concat!("Returns the raw value of this [`", stringify!($name), "`].")]
            #[inline]
            pub fn as_raw(self) -> $inner {
                self.0
            }

            #[doc = ::core::concat!("Returns whether this [`", stringify!($name), "`] is a known enum value.")]
            #[allow(clippy::manual_range_patterns)]
            pub fn is_known(self) -> bool {
                ::core::matches!(self.0, $(
                    | $value
                )*)
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self.0 {
                    $(
                        $value => write!(f, stringify!($variant)),
                    )*
                    _ => f.debug_tuple(stringify!($name)).field(&self.0).finish(),
                }
            }
        }
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
static BASE_REVISION: [u64; 3] = [0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, BASE_VERSION];

/// Returns whether the bootloader supports the base revision expected by the kernel.
#[inline]
pub fn base_revision_supported() -> bool {
    // We need to use volatile semantics to prevent the compiler from optimizing away the reads
    // of the `BASE_REVISION` array.
    //
    // This is needed because from the point of view of the compiler the array is never mutated.

    // SAFETY:
    //  We're reading a regular Rust reference, ensuring that this is safe.
    unsafe { core::ptr::read_volatile(&BASE_REVISION[2]) == 0u64 }
}

/// A request identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Id(pub [u64; 4]);

impl Id {
    pub const BOOTLOADER_INFO: Self = Self::common(0xf55038d8e2a1202f, 0x279426fcf5f59740);
    pub const ENTRY_POINT: Self = Self::common(0x13d86c035a1cd3e1, 0x2b0caa89d8f3026a);
    pub const MEMMAP: Self = Self::common(0x67cf3d9d378a806f, 0xe304acdfc50c3c62);
    pub const HHDM: Self = Self::common(0x48dcf1cb8ad2b852, 0x63984e959a98244b);
    pub const KERNEL_ADDRESS: Self = Self::common(0x71ba76863cc55f63, 0xb2644a48c516a487);
    pub const MODULE: Self = Self::common(0x3e7e279702be32af, 0xca1c4f3bd1280cee);

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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootloaderInfoRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<BootloaderInfoResponse>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootloaderInfoResponse {
    pub revision: Revision,
    pub name: LiminePtr<c_char>,
    pub version: LiminePtr<c_char>,
}

pub type EntryPoint = unsafe extern "C" fn() -> !;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct EntryPointRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<EntryPointResponse>,
    pub entry: EntryPoint,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct EntryPointResponse {
    pub revision: Revision,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MemmapRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<MemmapResponse>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MemmapResponse {
    pub revision: Revision,
    pub entry_count: u64,
    pub entries: LiminePtr<LiminePtr<MemmapEntry>>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MemmapEntry {
    pub base: u64,
    pub length: u64,
    pub ty: MemmapType,
}

create_loose_enum! {
    pub struct MemmapType: u32 {
        const USABLE = 0;
        const RESERVED = 1;
        const ACPI_RECLAIMABLE = 2;
        const ACPI_NVS = 3;
        const BAD_MEMORY = 4;
        const BOOTLOADER_RECLAIMABLE = 5;
        const KERNEL_AND_MODULES = 6;
        const FRAMEBUFFER = 7;
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct HhdmRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<HhdmResponse>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct HhdmResponse {
    pub revision: Revision,
    pub offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct KernelAddressRequest {
    pub id: Id,
    pub revision: Revision,
    pub response: ResponsePtr<KernelAddressResponse>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct KernelAddressResponse {
    pub revision: Revision,
    pub physical_base: u64,
    pub virtual_base: u64,
}

pub struct InternalModule {
    pub path: LiminePtr<c_char>,
    pub cmdline: LiminePtr<c_char>,
    pub flags: InternalModuleFlags,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InternalModuleFlags: u64 {
        const REQUIRED = 1 << 0;
    }
}

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
    pub media_type: MediaType,
    pub unused: u32,
    pub tftp_ip: u32,
    pub tftp_port: u32,
    pub partition_index: u32,
    pub mbr_disk_id: u32,
    pub gpt_disk_uuid: Uuid,
    pub gpt_part_uuid: Uuid,
    pub part_uuid: Uuid,
}

create_loose_enum! {
    pub struct MediaType: u64 {
        const GENERIC = 0;
        const OPTICAL = 1;
        const TFTP = 2;
    }
}
