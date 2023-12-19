/// An error that might occur while parsing an ELF file.
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// The header is not properly aligned for the transmutation.
    MisalignedHdr,
    /// The header is specified outside of the file.
    HdrOutsideFile,
    /// The program headers are not properly aligned for the transmutation.
    MisalignedPhdrs,
    /// The program headers are specified outside of the file.
    PhdrsOutsideFile,
    /// The program header size reported in the header file is invalid.
    InvalidPhdrSize,
}
