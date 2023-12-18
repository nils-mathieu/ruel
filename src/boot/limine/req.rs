//! This modules uses the raw structures defined in [`raw`] to provide a more ergonomic interface
//! to the answers the bootloader provided to the kernel.

use super::raw::*;

#[used(linker)]
static ENTRY_POINT: EntryPointRequest = EntryPointRequest {
    id: ENTRY_POINT_REQUEST,
    revision: 0,
    response: ResponsePtr::NULL,
    entry: super::main,
};

#[used(linker)]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest {
    id: BOOTLOADER_INFO_REQUEST,
    revision: 0,
    response: ResponsePtr::NULL,
};

#[used(linker)]
static MEMMAP_ENTRY: MemmapRequest = MemmapRequest {
    id: MEMMAP_REQUEST,
    revision: 0,
    response: ResponsePtr::NULL,
};

/// Stores information about the bootloader, including its name and version.
#[derive(Clone)]
pub struct BootloaderInfo<'a> {
    /// The name of the bootloader.
    pub name: &'a [u8],
    /// The version of the bootloader.
    pub version: &'a [u8],
}

/// A token that vouchers for common assumptions that the Kernel has to make in order to
/// access the data provided by the bootloader.
///
/// More information in the safety requirements section of [`Token::get`].
#[derive(Clone, Copy)]
pub struct Token<'a> {
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> Token<'a> {
    /// Creates a new [`Token`] instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because creating a [`Token`] instance requires the caller to
    /// ensure that a few assumptions are true:
    ///
    /// - The requests embedded in the kernel's binary have been answered by a bootloader that
    ///   complies with the Limine boot protocol. Or not answered at all.
    ///
    /// - The bootloader-reclaimable memory region is not overwritten.
    ///
    /// - The base revision expected by the kernel must be supported by the bootloader.
    ///
    /// The created [`Token`] instance logically "borrows" the bootloader-reclaimable memory
    /// in its entirety for the lifetime `'a`, meaning that accessing it mutably is no longer
    /// allowed until the [`Token`] instance is dropped.
    #[inline]
    pub const unsafe fn get() -> Self {
        Self {
            _marker: core::marker::PhantomData,
        }
    }

    /// Returns the response that the bootloader provided to the kernel for the bootloader info
    /// request.
    pub fn bootloader_info(self) -> Option<BootloaderInfo<'a>> {
        let response = unsafe { BOOTLOADER_INFO.response.read()? };

        Some(BootloaderInfo {
            name: unsafe { response.name.as_cstr().to_bytes() },
            version: unsafe { response.version.as_cstr().to_bytes() },
        })
    }

    /// Returns the response that the bootloader provided to the kernel for the entry point
    /// request.
    pub fn entry_point(self) -> Option<&'a EntryPointResponse> {
        unsafe { ENTRY_POINT.response.read() }
    }

    /// Returns the memory map entries provided by the bootloader.
    pub fn memmap(&self) -> Option<&'a [&'a MemmapEntry]> {
        let response = unsafe { MEMMAP_ENTRY.response.read()? };
        Some(unsafe { response.entries.cast().slice(response.entry_count as usize) })
    }
}
