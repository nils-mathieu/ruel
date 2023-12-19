use x86_64::{page_align_down, page_align_up, PageTableEntry, VirtAddr};

use crate::boot::{handle_mapping_error, oom};
use crate::cpu::paging::FOUR_KIB;
use crate::global::GlobalToken;
use crate::hcf::die;
use crate::log;
use crate::process::{Process, USERLAND_STOP};

/// Loads an ELF process from the provided file.
pub fn load(file: &[u8], cmdline: &[u8]) -> Process {
    log::trace!("Loading the init process from and ELF file...");

    let glob = GlobalToken::get();
    let mut process = Process::empty(glob).unwrap_or_else(|_| oom());

    let elf_file = elf::Elf::new(file);
    let hdr = elf_file.header().unwrap_or_else(|err| panic_parse(err));

    // =============================================================================================
    // SANITY CHECKS
    // =============================================================================================

    // This should be handled by the caller.
    assert_eq!(hdr.magic, [0x7f, 0x45, 0x4c, 0x46]);

    if hdr.class != elf::Class::ELF64 {
        log::error!("The provided init process is not a 64-bit ELF file.");
        die();
    }

    if hdr.data_encoding != elf::DataEncoding::NATIVE {
        log::error!(
            "\
            The provided init process does not have the right endianess.\n\
            Expected: {:?}; got: {:?}\
            ",
            elf::DataEncoding::NATIVE,
            hdr.data_encoding
        );
        die();
    }

    if hdr.elf_version != elf::Version::CURRENT {
        log::error!(
            "\
            The provided init process does not have the right ELF version.\n\
            Expected: {:?}; got: {:?}\
            ",
            elf::Version::CURRENT,
            hdr.elf_version
        );
        die();
    }

    if hdr.os_abi != elf::OsAbi::SYSV {
        log::error!(
            "\
            The provided init process does not have the right OS ABI.\n\
            Expected: {:?}; got: {:?}\
            ",
            elf::OsAbi::SYSV,
            hdr.os_abi
        );
        die();
    }

    if hdr.ty == elf::Type::DYN {
        log::error!(
            "\
            The provided init process is a relocatable file. You probably compiled your\n\
            init process as a Position-Independent Executable (PIE), which is not supported\n\
            by the kernel.\n\
            \n\
            Please compile the init process with a static relocation model.\
            "
        );
        die();
    }

    if hdr.ty != elf::Type::EXEC {
        log::error!(
            "\
            The provided init process is not an executable.\n\
            Expected: {:?}; got: {:?}\
            ",
            elf::Type::EXEC,
            hdr.ty
        );
        die();
    }

    if hdr.machine != elf::Machine::X86_64 {
        log::error!(
            "\
            The provided init process is not an x86_64 executable.\n\
            Expected: {:?}; got: {:?}\
            ",
            elf::Machine::X86_64,
            hdr.machine
        );
        die();
    }

    if hdr.entry_point == 0 {
        log::error!("The provided init process has no entry point specified.");
        die();
    }

    process.registers.rip = hdr.entry_point as VirtAddr;

    // =============================================================================================
    // LOAD SEGMENTS
    // =============================================================================================

    let mut stack_flags = None::<PageTableEntry>;

    for phdr in elf_file
        .program_headers()
        .unwrap_or_else(|err| panic_parse(err))
    {
        match phdr.ty {
            elf::PhdrType::NULL | elf::PhdrType::PHDR => (),
            elf::PhdrType::GNU_STACK => {
                if stack_flags.is_some() {
                    custom_panic_parse("multiple GNU_STACK segments");
                }

                stack_flags = Some(phdr_to_page_flags(phdr.flags));
            }
            elf::PhdrType::LOAD => load_segment(phdr, file, &mut process),
            unknown => {
                log::warn!(
                    "Found an unsupported segment type in the init process ELF file: {unknown:?}\n\
                    This segment will be ignored."
                );
                continue;
            }
        }
    }

    // =============================================================================================
    // ALLOCATE STACK
    // =============================================================================================

    // Make sure that the command line can be copied into the stack.
    if cmdline.len() + 1 >= 4096 {
        log::error!(
            "\
            The provided command line is larger than 4096 bytes. Larger command-line are not\n\
            supported by the kernel due to the laziness of the kernel developers.\
            ",
        );
        die();
    }

    let stack_flags = stack_flags.unwrap_or(
        PageTableEntry::USER_ACCESSIBLE | PageTableEntry::WRITABLE | PageTableEntry::NO_EXECUTE,
    );

    // Allocate a stack for the process.
    const STACK_SIZE: usize = 8 * FOUR_KIB;
    const STACK_POS: VirtAddr = 1 + USERLAND_STOP - STACK_SIZE - FOUR_KIB;
    #[allow(clippy::assertions_on_constants)]
    const _: () = assert!(STACK_POS & 0xFFF == 0);

    let cmdline_start = STACK_POS + STACK_SIZE - cmdline.len() - 1;

    process
        .address_space
        .allocate_range(STACK_POS, STACK_SIZE, stack_flags, |virt, dst| {
            if virt == STACK_POS + STACK_SIZE - 0x1000 {
                // This is the last page of the stack. Copy the command line into it.
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        cmdline.as_ptr(),
                        dst.add(cmdline_start - virt),
                        cmdline.len(),
                    );
                    core::ptr::write(dst.add(cmdline_start - virt + cmdline.len()), 0);
                }
            }
        })
        .unwrap_or_else(|err| handle_mapping_error(err));

    // The command-line string has been copied at the top of the stack, meaning that the stack
    // starts right after it.
    process.registers.rsp = cmdline_start & !0xF;
    process.registers.rbp = cmdline_start & !0xF;
    process.registers.rdi = cmdline_start;

    process
}

/// Loads a segment into the process' memory.
fn load_segment(segment: &elf::Phdr, file: &[u8], process: &mut Process) {
    let flags = phdr_to_page_flags(segment.flags);

    if segment.align != FOUR_KIB as u64 {
        custom_panic_parse("segment alignment is not 4KiB");
    }

    if segment.vaddr & 0xFFF != segment.offset & 0xFFF {
        custom_panic_parse("segment virtual address and offset are not aligned");
    }

    if segment.filesz > segment.memsz {
        custom_panic_parse("segment file size is larger than its memory size");
    }

    if segment.offset.saturating_add(segment.filesz) > file.len() as u64 {
        custom_panic_parse("segment file size is larger than the file size");
    }

    let page_start = page_align_down(segment.vaddr as usize);
    let page_end = page_align_up(segment.vaddr as usize + segment.memsz as usize);

    let virt_to_file = segment.offset.wrapping_sub(segment.vaddr);

    process
        .address_space
        .allocate_range(page_start, page_end - page_start, flags, |virt, dst| {
            let mem_start = (segment.vaddr as usize).max(virt);
            let mem_end = (segment.vaddr as usize + segment.memsz as usize).min(virt + FOUR_KIB);

            let file_start = mem_start.wrapping_add(virt_to_file as usize);
            let mut file_end = mem_end.wrapping_add(virt_to_file as usize);
            let mut zeroed = 0;

            if file_start > (segment.offset + segment.filesz) as usize {
                zeroed = mem_end - mem_start;
                file_end = file_start;
            } else if file_end > (segment.offset + segment.filesz) as usize {
                zeroed = file_end - (segment.offset + segment.filesz) as usize;
                file_end = (segment.offset + segment.filesz) as usize;
            }

            unsafe {
                debug_assert!((mem_start - virt) + (file_end - file_start) <= FOUR_KIB);

                core::ptr::copy_nonoverlapping(
                    file.as_ptr().add(file_start),
                    dst.add(mem_start - virt),
                    file_end - file_start,
                );

                core::ptr::write_bytes(
                    dst.add((mem_end - virt) + (file_end - file_start) - zeroed),
                    0,
                    zeroed,
                );
            }
        })
        .unwrap_or_else(|err| handle_mapping_error(err));
}

/// Converts an ELF program header flags to page table flags.
fn phdr_to_page_flags(flags: elf::PhdrFlags) -> PageTableEntry {
    // FIXME: Figure out why NO_EXECUTE breaks everything.

    let mut out = /* PageTableEntry::NO_EXECUTE | */ PageTableEntry::USER_ACCESSIBLE;

    if !flags.intersects(elf::PhdrFlags::READABLE) {
        log::warn!(
            "Found a non-readable segment in the init process.\n\
            The segment will be readable anyway."
        );
    }

    if flags.intersects(elf::PhdrFlags::WRITABLE) {
        out.insert(PageTableEntry::WRITABLE);
    }

    // if !flags.intersects(elf::PhdrFlags::EXECUTABLE) {
    //     out.remove(PageTableEntry::NO_EXECUTE);
    // }

    out
}

/// Prints an message indicating that the ELF file is invalid and dies.
fn panic_parse(err: elf::Error) -> ! {
    log::trace!("The init process is an invalid ELF file: {:?}", err);
    die();
}

/// Prints an message indicating that the ELF file is invalid and dies.
fn custom_panic_parse(err: &str) -> ! {
    log::trace!("The init process is an invalid ELF file: {}", err);
    die();
}
