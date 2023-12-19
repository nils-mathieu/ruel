/// An error that might occur while parsing an ELF file.
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// The file is not properly aligned and cannot be properly transmuted.
    Misaligned,
    /// The file is too small to be a valid ELF file.
    TooSmallToBeElf,
    /// The program headers are not properly aligned for the transmutation.
    MisalignedPhdrs,
    /// The program headers are specified outside of the file.
    PhdrsOutsideFile,
    /// The program header size reported in the header file is invalid.
    InvalidPhdrSize,
}
