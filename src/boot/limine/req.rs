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

/// A token that vouchers for common assumptions that the Kernel has to make in order to
/// access the data provided by the bootloader.
///
/// More information in the safety requirements section of [`Token::get`].
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

    /// Returns the response that the bootloader provided to the kernel for the entry point
    /// request.
    #[inline]
    pub fn entry_point(self) -> Option<&'a EntryPointResponse> {
        unsafe { ENTRY_POINT.response.read() }
    }
}
