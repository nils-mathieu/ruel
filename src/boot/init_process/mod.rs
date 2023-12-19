//! Provides ways to load the init process into memory.

use crate::hcf::die;
use crate::log;
use crate::process::Process;

#[cfg(feature = "init-elf")]
mod elf;

/// A possible file type, used to determine how to load the file into memory.
enum FileType {
    /// The type of the file is unknown.
    Unknown,
    /// The file is an ELF file.
    Elf,
}

impl FileType {
    /// Attempts to determine the type of the provided file.
    pub fn of(file: &[u8]) -> Self {
        if file.starts_with(b"\x7fELF") {
            Self::Elf
        } else {
            Self::Unknown
        }
    }
}

/// Loads a process from the provided file.
pub fn load_any(file: &[u8], cmdline: &[u8]) -> Process {
    match FileType::of(file) {
        FileType::Elf => elf::load(file, cmdline),
        FileType::Unknown => {
            log::error!(
                "\
                The type of the init process could not be determined.\n\
                The kernel won't be able to start it.\n\
                \n\
                Supported file foramts are:\n\
                - ELF\
                "
            );
            die();
        }
    }
}
