//! This crate provides an implementation of the [Limine Boot Protocol](PROTOCOL).
//!
//! [PROTOCOL]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md

#![no_std]
#![feature(used_with_arg)]
#![forbid(unsafe_op_in_unsafe_fn)]

use bitflags::bitflags;
use core::ffi::c_char;

mod limine_ptr;
pub use limine_ptr::*;

/// Creates a type that acts like an enum, but internally allows every bit patterns (unknown
/// values). This makes the library safer than using a regular enum, as it prevents
/// undefined behavior in case the bootloader sends an unknown value for some reason (for example
/// because it uses a version that we do not support).
///
/// The syntax is basically the same as the [`bitflags!`] macro.
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
    /// The ID to use with the [`BootloaderInfoRequest`].
    pub const BOOTLOADER_INFO: Self = Self::common(0xf55038d8e2a1202f, 0x279426fcf5f59740);
    /// The ID to use with the [`EntryPointRequest`].
    pub const ENTRY_POINT: Self = Self::common(0x13d86c035a1cd3e1, 0x2b0caa89d8f3026a);
    /// The ID to use with the [`MemmapRequest`].
    pub const MEMMAP: Self = Self::common(0x67cf3d9d378a806f, 0xe304acdfc50c3c62);
    /// The ID to use with the [`HhdmRequest`].
    pub const HHDM: Self = Self::common(0x48dcf1cb8ad2b852, 0x63984e959a98244b);
    /// The ID to use with the [`KernelAddressRequest`].
    pub const KERNEL_ADDRESS: Self = Self::common(0x71ba76863cc55f63, 0xb2644a48c516a487);
    /// The ID to use with the [`ModuleRequest`].
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

/// Requests the bootloader to provide some information about itself. That includes its name
/// and version.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootloaderInfoRequest {
    /// Must be [`Id::BOOTLOADER_INFO`].
    pub id: Id,
    /// The revision of the request.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,
    /// The response pointer of the request.
    ///
    /// More information in the documentation for [`ResponsePtr`].
    pub response: ResponsePtr<BootloaderInfoResponse>,
}

/// The response to the [`BootloaderInfoRequest`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootloaderInfoResponse {
    /// The revision of the response.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,

    /// A null-terminated ASCII string containing the name of the bootloader.
    pub name: LiminePtr<c_char>,
    /// A null-terminated ASCII string containing the version of the bootloader.
    pub version: LiminePtr<c_char>,
}

/// The function signature that Limine expects when booting the kernel up.
pub type EntryPoint = unsafe extern "C" fn() -> !;

/// Requests the bootloader to ignore the entry point specified in the kernel's ELF header and
/// instead use the provided entry point.
///
/// This is useful for kernels that support multiple boot protocols and want to use distinct
/// entry points for each of them.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EntryPointRequest {
    /// Must be [`Id::ENTRY_POINT`].
    pub id: Id,
    /// The revision number of the request.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,
    /// The response pointer of the request.
    ///
    /// More information in the documentation for [`ResponsePtr`].
    pub response: ResponsePtr<EntryPointResponse>,

    /// The entry point function to call in order to give control to the kernel.
    pub entry: EntryPoint,
}

/// The response to the [`EntryPointRequest`].
///
/// This response contains no additional information, it can be used by the bootloader to
/// signal the kernel that the request was successful (which is given by the fact that the
/// code can even run).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EntryPointResponse {
    /// The revision number of the response.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,
}

/// Requests the bootloader to provide a map of the physical memory available on the system.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemmapRequest {
    /// Must be [`Id::MEMMAP`].
    pub id: Id,
    /// The revision number of the request.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,
    /// The response pointer of the request.
    ///
    /// More information in the documentation for [`ResponsePtr`].
    pub response: ResponsePtr<MemmapResponse>,
}

/// The response to the [`MemmapRequest`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemmapResponse {
    /// The revision number of the response.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,

    /// The number of entries referenced by `entries`.
    pub entry_count: u64,
    /// The entries of the memory map.
    ///
    /// The entries in this list are guaranteed to:
    ///
    /// 1. Be sorted in ascending order by their base address.
    ///
    /// 2. Usable memory and bootloader-reclaimable memory entries are guaranteed to not overlap
    ///    with any other entries, and to be aligned to a page boundary (4KiB).
    pub entries: LiminePtr<LiminePtr<MemmapEntry>>,
}

/// An memory map entry, available through the [`MemmapRequest`].
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemmapEntry {
    /// The base physical address of the region.
    pub base: u64,
    /// The length of the region.
    pub length: u64,
    /// The type of the region.
    pub ty: MemmapType,
}

create_loose_enum! {
    /// The type of a [`MemmapEntry`].
    pub struct MemmapType: u32 {
        /// The memory region is usable.
        const USABLE = 0;
        /// The memory region is reserved and cannot be used.
        const RESERVED = 1;

        // Not sure what this is.
        const ACPI_RECLAIMABLE = 2;
        const ACPI_NVS = 3;

        /// The memory region is not usable.
        const BAD_MEMORY = 4;

        /// The memory region is usable, but the bootloader used it to store some data, including
        /// the responses it sent to the kernel, the current GDT (on x86_64), as well as the
        /// kernel's stack.
        const BOOTLOADER_RECLAIMABLE = 5;

        /// The memory region is usable, but the bootloader used it to store either the kernel or
        /// one of the loaded modules.
        const KERNEL_AND_MODULES = 6;

        /// The memory region is used by a framebuffer.
        const FRAMEBUFFER = 7;
    }
}

/// Requests the bootloader to provide the address of the HHDM (Higher-Half Direct Map) that it
/// has created for the kernel.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HhdmRequest {
    /// Must be [`Id::HHDM`].
    pub id: Id,
    /// The revision number of the request.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,
    /// The response pointer of the request.
    ///
    /// More information in the documentation for [`ResponsePtr`].
    pub response: ResponsePtr<HhdmResponse>,
}

/// The response to the [`HhdmRequest`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HhdmResponse {
    /// The revision number of the response.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,

    /// The base virtual address of the HHDM.
    ///
    /// Writing to a pointer larger than this value will write at physical address
    /// `(pointer - offset)`.
    pub offset: u64,
}

/// Requests the kernel to provide the address of the kernel's physical and virtual base.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KernelAddressRequest {
    /// Must be [`Id::KERNEL_ADDRESS`].
    pub id: Id,
    /// The revision number of the request.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,
    /// The response pointer of the request.
    ///
    /// More information in the documentation for [`ResponsePtr`].
    pub response: ResponsePtr<KernelAddressResponse>,
}

/// The response ot the [`KernelAddressRequest`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KernelAddressResponse {
    /// The revision number of the response.
    ///
    /// Currently, only revision 0 exists.
    pub revision: Revision,

    /// The physical base of the kernel image.
    pub physical_base: u64,
    /// The virtual base of the kernel image.
    pub virtual_base: u64,
}

/// An internal module that the kernel can request from the bootloader.
///
/// When an internal module is specified, the bootloader always loads it before the modules
/// that the user requested.
pub struct InternalModule {
    /// The path of the module.
    pub path: LiminePtr<c_char>,
    /// The command-line arguments to pass to the module.
    pub cmdline: LiminePtr<c_char>,
    /// Some flags associated with this entry.
    pub flags: InternalModuleFlags,
}

bitflags! {
    /// Some flags associated with an [`InternalModule`].
    #[repr(transparent)]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InternalModuleFlags: u64 {
        /// Whether the internal module is required.
        ///
        /// The bootloader will fail to load the kernel if the module is not found.
        const REQUIRED = 1 << 0;
    }
}

/// Requests the bootloader to provide the list of the modules that were loaded; either because
/// the user requested them, or because the kernel requested them through the internal
/// module mechanism.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModuleRequest {
    /// Must be [`Id::MODULE`].
    pub id: Id,
    /// The revision number of the request.
    ///
    /// # Revision 0
    ///
    /// With revision 0, no additional fields can be used. The list of returned modules is
    /// the list of modules that the user provided in the `limine.cfg` file.
    ///
    /// # Revision 1
    ///
    /// With revision 1, the bootloader allows the kernel to request some modules as well. The
    /// `internal_module_count` and `internal_modules` fields are used to specify the list of
    /// modules that the kernel wants to load.
    pub revision: Revision,
    pub response: ResponsePtr<ModuleResponse>,

    //
    // Request revision 1
    //
    /// The number of entries pointed by the `internal_modules` pointer.
    pub internal_module_count: u64,
    /// The internal modules that the kernel wants to load.
    pub internal_modules: LiminePtr<LiminePtr<InternalModule>>,
}

/// The response to the [`ModuleRequest`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModuleResponse {
    /// The revision number of the response.
    ///
    /// # Revision 0
    ///
    /// If the bootloader uses revision 0, then the internal modules eventually requested by the
    /// kernel are ignored.
    ///
    /// # Revision 1
    ///
    /// If the bootloader uses revision 1, then internal modules requested by the kernel are
    /// honored and loaded *before* the modules requested by the user.
    pub revision: Revision,

    /// The number of entries pointed by the `modules` pointer.
    pub module_count: u64,
    /// The modules that were loaded, in the order they were declared in the `limine.cfg` file.
    ///
    /// If internal modules were requested by the kernel, and if the revision number is at least
    /// 1, then the internal modules are loaded *before* the modules requested by the user.
    pub modules: LiminePtr<LiminePtr<File>>,
}

/// A universally unique identifier.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uuid(u32, u16, u16, [u8; 8]);

/// A file that was loaded by the bootloader.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct File {
    /// The revision number of the file.
    ///
    /// Currently, only revision 0 exists.
    pub revision: u64,

    /// The virtual address of the file in memory.
    ///
    /// This address already has the HHDM offset applied to it.
    pub address: LiminePtr<u8>,
    /// The size of the file in memory.
    pub size: u64,

    /// The path to the file.
    pub path: LiminePtr<c_char>,

    /// The command-line arguments passed to the file, if applicable.
    pub cmdline: LiminePtr<c_char>,

    /// The media type used to store the file.
    pub media_type: MediaType,

    pub unused: u32,

    /// If the file was loaded from a TFTP server, the IP address of the server.
    pub tftp_ip: u32,
    /// If the file was loaded from a TFTP server, the port of the server.
    pub tftp_port: u32,

    /// The 1-based partition index of the volume from which the file was loaded.
    ///
    /// If 0, the volume is invalid or unpartitioned.
    pub partition_index: u32,

    /// If non-zero, the ID of the disk the file was loaded from as reported in the MBR.
    pub mbr_disk_id: u32,

    /// If non-zero, the UUID of the disk the file was loaded from as reported in its GPT.
    pub gpt_disk_uuid: Uuid,
    /// If non-zero, the UUID of the partition the file was loaded from as reported in its GPT.
    pub gpt_part_uuid: Uuid,

    /// If non-zero, the UUID of the filesystem the file was loaded from.
    pub part_uuid: Uuid,
}

create_loose_enum! {
    /// The type of media used to store a [`File`].
    pub struct MediaType: u64 {
        const GENERIC = 0;
        const OPTICAL = 1;
        const TFTP = 2;
    }
}
