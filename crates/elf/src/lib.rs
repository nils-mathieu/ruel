//! A simple ELF parser.

#![no_std]

mod error;
use core::mem::align_of;
use core::mem::size_of;

pub use self::error::*;

mod abi;
pub use self::abi::*;

/// A parsable ELF file.
#[derive(Clone, Copy)]
pub struct Elf<'a> {
    /// The bytes of the file.
    bytes: &'a [u8],
}

impl<'a> Elf<'a> {
    /// Creates a new [`Elf`] instance from the provided bytes.
    #[inline]
    pub const fn new(file: &'a [u8]) -> Self {
        Self { bytes: file }
    }

    /// Returns the header of the file.
    pub fn header(self) -> Result<&'a Ehdr, Error> {
        if self.bytes.len() < size_of::<Ehdr>() {
            return Err(Error::HdrOutsideFile);
        }

        if self.bytes.as_ptr() as usize % align_of::<Ehdr>() != 0 {
            return Err(Error::MisalignedHdr);
        }

        unsafe { Ok(&*(self.bytes.as_ptr() as *const Ehdr)) }
    }

    /// Returns the program headers of the file.
    pub fn program_headers(self) -> Result<&'a [Phdr], Error> {
        let hdr = self.header()?;

        if size_of::<Phdr>() != hdr.phentsize as usize {
            return Err(Error::InvalidPhdrSize);
        }

        let start = hdr.phoff as usize;
        let end = start
            .checked_add(hdr.phnum as usize)
            .ok_or(Error::PhdrsOutsideFile)?
            .checked_mul(hdr.phentsize as usize)
            .ok_or(Error::PhdrsOutsideFile)?;

        if self.bytes.len() < end {
            return Err(Error::PhdrsOutsideFile);
        }

        if start % align_of::<Phdr>() != 0 {
            return Err(Error::MisalignedPhdrs);
        }

        unsafe {
            Ok(core::slice::from_raw_parts(
                self.bytes.as_ptr().add(start) as *const Phdr,
                hdr.phnum as usize,
            ))
        }
    }
}
