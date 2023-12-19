use bitflags::bitflags;

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

create_loose_enum! {
    /// The class of an ELF file.
    pub struct Class: u8 {
        /// A 32-bit ELF file.
        const ELF32 = 1;
        /// A 64-bit ELF file.
        const ELF64 = 2;
    }
}

create_loose_enum! {
    /// The data encoding of an ELF file.
    pub struct DataEncoding: u8 {
        /// Little endian, two's complement.
        const LITTLE_ENDIAN = 1;
        /// Big endian, two's complement.
        const BIG_ENDIAN = 2;
    }
}

impl DataEncoding {
    /// The current data encoding.
    #[cfg(target_endian = "little")]
    pub const NATIVE: Self = Self::LITTLE_ENDIAN;
    /// The current data encoding.
    #[cfg(target_endian = "big")]
    pub const NATIVE: Self = Self::BIG_ENDIAN;
}

create_loose_enum! {
    /// The version of ELF that a file uses.
    pub struct Version: u8 {
        /// The current version.
        const CURRENT = 1;
    }
}

create_loose_enum! {
    /// The operating system and ABI to which the object is targeted.
    pub struct OsAbi: u8 {
        const SYSV = 0;
        const LINUX = 3;
        const STANDALONE = 255;
    }
}

create_loose_enum! {
    /// The type of an ELF file.
    pub struct Type: u16 {
        /// An unknown type.
        const NONE = 0;
        /// A relocatable file.
        const REL = 1;
        /// An executable file.
        const EXEC = 2;
        /// A shared object.
        const DYN = 3;
        /// A core file.
        const CORE = 4;
    }
}

create_loose_enum! {
    /// The required architecture of an ELF file.
    pub struct Machine: u16 {
        /// An unknown architecture.
        const NONE = 0;

        /// The x86_64 architecture.
        const X86_64 = 0x3e;
    }
}

/// The header of an ELF file.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ehdr {
    /// An array of bytes that describe how to interpret the file.
    pub magic: [u8; 4],
    /// The class of the ELF file.
    pub class: Class,
    /// The data encoding used by the ELF file.
    pub data_encoding: DataEncoding,
    /// The version of ELF that the file uses.
    pub elf_version: Version,
    /// The operating system and ABI to which the object is targeted.
    pub os_abi: OsAbi,
    /// The version of the above ABI.
    pub abi_version: u8,

    #[doc(hidden)]
    pub _padding: [u8; 7],

    /// The type of the ELF file.
    pub ty: Type,
    /// The machine architecture that the file is encoded for.
    pub machine: Machine,
    /// The file version.
    pub version: u32,

    /// The virtual address of the entry point of the program.
    pub entry_point: u64,

    /// The offset of the program header table within the file.
    pub phoff: u64,
    /// The offset of the section header table within the file.
    pub shoff: u64,

    /// Some processor-specific flags.
    /// Currently, no flags are defined.
    pub flags: u32,

    /// The size of the elf header, in bytes.
    pub ehsize: u16,

    /// The size of one entry in the program header table.
    pub phentsize: u16,
    /// The number of entries in the program header table.
    pub phnum: u16,

    /// The size of one entry in the section header table.
    pub shentsize: u16,
    /// The number of entries in the section header table.
    pub shnum: u16,
    /// The index of the section header table entry that contains the string table.
    pub shstrndx: u16,
}

create_loose_enum! {
    /// The type of a program header.
    pub struct PhdrType: u32 {
        /// A null program header.
        ///
        /// The other fields should be ignored, the header is effectively unused.
        const NULL = 0;

        /// A loadable segment.
        ///
        /// Loadable segments must be loaded in memory by the loader.
        ///
        /// If the in-memory size of the segment is larger than its in-file size, the extra bytes
        /// must be zeroed out.
        ///
        /// The file size must not be larger than the memory size.
        ///
        /// Those segments normally appear in ascending order in the file, sorted by their
        /// `vaddr` field.
        const LOAD = 1;

        /// The segment provides dynamic linker information.
        const DYNAMIC = 2;

        /// The location and size of a null-terminated path name to invoke as an interpreter.
        ///
        /// Only one such segment may be present in the file.
        const INTERP = 3;

        /// The segment contains the location of notes.
        const NOTE = 4;

        /// The segment contains the location of a dynamic symbol table.
        const SHLIB = 5;

        /// The segment contains the location and size of the program header table itself.
        const PHDR = 6;

        /// If present, indicates that the the stack should mapped with the given permissions.
        const GNU_STACK = 0x6474e551;

        /// If present, indicates that the the read-only relocations should be made writable.
        const GNU_RELRO = 0x6474e552;
    }
}

bitflags! {
    /// Describes how a segment should be accessed.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct PhdrFlags: u32 {
        /// The segment is executable.
        const EXECUTABLE = 1 << 0;
        /// The segment is writable.
        const WRITABLE = 1 << 1;
        /// The segment is readable.
        const READABLE = 1 << 2;
    }
}

/// A program header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Phdr {
    /// The kind of the segment described by this header.
    pub ty: PhdrType,

    /// The access flags of the segment.
    pub flags: PhdrFlags,

    /// The offset of the segment within the file.
    pub offset: u64,

    /// The virtual address where the segment must be loaded in memory.
    pub vaddr: u64,
    /// The physical address where the segment must be loaded in memory.
    ///
    /// This can usually be ignored.
    pub paddr: u64,

    /// The size of the segment in the file.
    pub filesz: u64,
    /// The size of the segment in memory.
    pub memsz: u64,

    /// The alignment of the segment in memory.
    ///
    /// This must be a power of two.
    ///
    /// There is always the following constraint: `vaddr % align == offset % align`.
    pub align: u64,
}
