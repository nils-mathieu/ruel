//! Defines the entry point of the kernel when it is booted by a Limine-compliant bootloader.
//!
//! This is a simple implementation of the [Limine boot protocol][PROTOCOL].
//!
//! [PROTOCOL]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md
//!
//! # Version
//!
//! The version 6 of the protocol is implemented.

use self::raw::{MemmapEntry, MemmapType};
use crate::cpu::paging::raw::{PageTable, PAGE_GLOBAL, PAGE_WRITE};
use crate::cpu::paging::{AddressSpace, AddressSpaceContext, MappingError, HHDM_OFFSET};
use crate::hcf::die;
use crate::log;
use crate::mem::{BumpAllocator, OutOfMemory, PhysAddr, VirtAddr};

mod raw;
mod req;

/// The entry point of the kernel when it is booted by a Limine-compliant bootloader.
///
/// # Safety
///
/// - This function expects to be called by a Limine-compliant bootloader, meaning that the
///   machine must currently be in the state described in the [Entry Machine State] section
///   of the protocol.
///
/// - It must only be called once.
///
/// [Entry Machine State]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md#entry-memory-layout
unsafe extern "C" fn main() -> ! {
    log::info!("Booting Ruel from the Limine entry point...");

    // =============================================================================================
    // Sanity Checks
    // =============================================================================================
    log::trace!("Performing some sanity checks...");

    if !raw::base_revision_supported() {
        log::error!(
            "\
            The bootloader does not support the base revision expected by the kernel.\n\
            This happens because you're bootloader is outdated.\n\
            \n\
            Please update your bootloader.\
            ",
        );
        die();
    }

    // SAFETY:
    //  We're at the beginning of the entry point function executed by the bootloader. The
    //  bootloader reclaimable memory region is still intact.
    let token = unsafe { req::Token::get() };

    if token.entry_point().is_none() {
        log::warn!(
            "\
            The bootloader did not respond to the `limine_entry_point_request` of the kernel.\n\
            This is a bug in the bootloader; the protocol requires it to respond to this\n\
            request.\
            ",
        );
    }

    if let Some(info) = token.bootloader_info() {
        log::info!(
            "Bootloader: {} (v{})",
            info.name.escape_ascii(),
            info.version.escape_ascii(),
        );
    } else {
        log::warn!(
            "\
            The bootloader did not respond to the `limine_bootloader_info_request` of the kernel.\n\
            This is not necessarily a bug in the bootloader; but it is pretty weird.\
            "
        );
    }

    let memory_map = token.memmap().unwrap_or_else(|| {
        log::error!(
            "\
            The bootloader did not respond to the `limine_memmap_request` of the kernel.\n\
            The kernel is unable to continue without a map of the memory regions.\
            "
        );
        die();
    });

    if memory_map.is_empty() {
        log::error!(
            "\
            The bootloader reported an empty memory map.\n\
            The kernel is unable to continue without a map of the memory regions.\
            "
        );
        die();
    }

    validate_memory_map(memory_map);

    let bootloader_hhdm = token
        .hhdm()
        .unwrap_or_else(|| {
            log::error!(
                "\
                The bootloader did not respond to the `limine_hhdm_request` of the kernel.\n\
                The kernel is unable to continue without knowing about the currently\n\
                active address space.\
                "
            );
            die();
        })
        .offset;

    log::trace!(
        "The HHDM provided by the bootloader is at {:#x}",
        bootloader_hhdm,
    );

    let kernel_address = token.kernel_address().unwrap_or_else(|| {
        log::error!(
            "\
            The bootloader did not respond to the `limine_kernel_address_request` of\n\
            the kernel. The kernel is unable to continue without knowing where it\n\
            has been loaded in physical memory.\
            "
        );
        die();
    });

    log::trace!(
        "The kernel has been loaded at {:#x} in physical memory.",
        kernel_address.physical_base
    );

    if kernel_address.virtual_base != crate::linker::kernel_image_begin() as u64 {
        log::error!(
            "\
            The kernel has been loaded at {:#x} in virtual memory, but the kernel\n\
            expected to be loaded at {:#x}. It's weird that this code is even managing\n\
            to execute?\
            ",
            kernel_address.virtual_base,
            crate::linker::kernel_image_begin() as u64,
        );
        die();
    }

    // =============================================================================================
    // Bootstrap Allocator
    // =============================================================================================
    log::trace!("Finding a suitable block for the bootstrap allocator...");

    let largest_usable_segment = find_largest_usable_segment(memory_map);

    log::trace!(
        "The bootstrap allocator will use the memory segment {:#x}..{:#x} ({})",
        largest_usable_segment.base,
        largest_usable_segment.base + largest_usable_segment.length,
        crate::utility::HumanByteCount(largest_usable_segment.length),
    );

    // SAFETY:
    //  The ownership of the largest usable segment is transferred to the bootstrap allocator. We
    //  won't be accessing it until the allocator is no longer used.
    let mut bootstrap_allocator = unsafe {
        BumpAllocator::new(
            largest_usable_segment.base,
            largest_usable_segment.base + largest_usable_segment.length,
        )
    };

    // =============================================================================================
    // CPU Initialization
    // =============================================================================================
    log::trace!("Creating the kernel address space...");

    let address_space = unsafe {
        create_kernel_address_space(
            &mut bootstrap_allocator,
            bootloader_hhdm as usize,
            kernel_address.physical_base,
            find_memory_upper_bound(memory_map),
        )
    };

    log::trace!("Kernel L4 Table stored at address {:#x}", address_space);

    todo!();
}

/// Finds the largest usable memory segment in the memory map.
///
/// # Panics
///
/// This function panics if `memory_map` is empty.
fn find_largest_usable_segment<'a>(memory_map: &[&'a MemmapEntry]) -> &'a MemmapEntry {
    memory_map
        .iter()
        .filter(|entry| entry.ty == MemmapType::USABLE)
        .max_by_key(|entry| entry.length)
        .unwrap()
}

/// Returns the upper bound of the memory region that is available on the system.
///
/// # Panics
///
/// This function panics if `memory_map` is empty.
fn find_memory_upper_bound(memory_map: &[&MemmapEntry]) -> PhysAddr {
    memory_map
        .iter()
        .filter(|entry| {
            entry.ty == MemmapType::USABLE || entry.ty == MemmapType::BOOTLOADER_RECLAIMABLE
        })
        .map(|entry| entry.base + entry.length)
        .max()
        .unwrap()
}

/// Validates the memory map provided by the bootloader.
///
/// If the map is found to break some of the invariants specified in the protocol, the function
/// stops the CPU.
fn validate_memory_map(memory_map: &[&MemmapEntry]) {
    let mut last_entry: Option<&MemmapEntry> = None;

    for entry in memory_map {
        if let Some(last_entry) = last_entry {
            if last_entry.base > entry.base {
                log::error!(
                    "\
                        The memory map provided by the bootloader is not sorted by base address.\n\
                        This is a bug in the bootloader; the protocol requires it to be already\n\
                        sorted.\
                        "
                );
                die();
            }
        }

        if entry.length == 0 {
            log::error!(
                "\
                The memory map provided by the bootloader contains an entry with a length of 0.\n\
                This is a bug in the bootloader; the protocol requires it to not contain such\n\
                entries.\
                "
            );
            die();
        }

        if entry.ty == MemmapType::USABLE || entry.ty == MemmapType::BOOTLOADER_RECLAIMABLE {
            if entry.base & 0xFFF != 0 || entry.length & 0xFFF != 0 {
                log::error!(
                    "\
                    The memory map provided by the bootloader contains a usable entry that\n\
                    is not page-aligned. This is a bug in the bootloader; the protocol requires it\n\
                    to be properly page-aligned.\
                    "
                );
                die();
            }

            if let Some(last_entry) = last_entry {
                if last_entry.base + last_entry.length > entry.base {
                    log::error!(
                        "\
                        The memory map provided by the bootloader contains overlapping usable\n\
                        entries. This is a bug in the bootloader; the protocol requires it to not\n\
                        contain such entries.\
                        "
                    );
                    die();
                }
            }
        }

        last_entry = Some(entry);
    }
}

/// Creates the address space of the kernel.
///
/// # Arguments
///
/// - `bootstrap_allocator` - The allocator used to allocate memory during the booting process.
///
/// - `hhdm` - The offset of the higher-half direct map currently in use. This mapping is usually
///   setup by the bootloader. If an identity mapping is used, this value should be `0`.
///
/// - `kernel_physical_base` - The physical address at which the kernel is loaded.
///
/// - `memory_upper_bound` - The upper bound of the memory available on the system.
///
/// # Errors
///
/// This function halts the CPU if it fails to allocate the memory required to create the address
/// space.
///
/// # Safety
///
/// This function assumes that the provided HHDM offset is valid.
pub unsafe fn create_kernel_address_space(
    boostrap_allocator: &mut BumpAllocator,
    hhdm: usize,
    kernel_physical_base: PhysAddr,
    memory_upper_bound: PhysAddr,
) -> PhysAddr {
    struct Context<'a> {
        allocator: &'a mut BumpAllocator,
        hhdm: usize,
    }

    unsafe impl AddressSpaceContext for Context<'_> {
        #[inline]
        fn allocate_page(&mut self) -> Result<PhysAddr, OutOfMemory> {
            self.allocator
                .allocate(core::alloc::Layout::new::<PageTable>())
        }

        #[inline]
        unsafe fn physical_to_virtual(&self, addr: PhysAddr) -> VirtAddr {
            addr as usize + self.hhdm
        }

        unsafe fn deallocate_page(&mut self, _addr: PhysAddr) {
            panic!("this `AddressSpaceContext` implementation does not support deallocations");
        }
    }

    let mut address_space = AddressSpace::new(Context {
        allocator: boostrap_allocator,
        hhdm,
    })
    .unwrap_or_else(|_| oom());

    // Create a direct mapping of the system's available memory (our very own higher half
    // direct map).
    address_space
        .map_range(
            HHDM_OFFSET,
            0,
            memory_upper_bound as usize,
            PAGE_WRITE | PAGE_GLOBAL,
        )
        .unwrap_or_else(|err| handle_mapping_error(err));

    // Map the kernel's physical memory into the address space, at the position that's specified
    // in the linker script.
    address_space
        .map_range(
            crate::cpu::paging::raw::align_down(crate::linker::kernel_image_begin() as VirtAddr),
            crate::cpu::paging::raw::align_down(kernel_physical_base as usize) as PhysAddr,
            crate::cpu::paging::raw::align_up(crate::linker::kernel_image_size()),
            PAGE_WRITE | PAGE_GLOBAL,
        )
        .unwrap_or_else(|err| handle_mapping_error(err));

    address_space.leak()
}

/// Handles a mapping error.
fn handle_mapping_error(err: MappingError) -> ! {
    match err {
        MappingError::AlreadyMapped => panic!("attempted to map a page that is already mapped"),
        MappingError::OutOfMemory => oom(),
    }
}

/// Prints an helpful message and halts the CPU.
fn oom() -> ! {
    log::error!(
        "\
        The system ran out of memory while booting up. This is likely due to a bug in the\n\
        kernel, but your system might just be missing the memory required to boot.\n\
        \n\
        If you believe that this is an error, please file an issue on the GitHub repository!\n\
        \n\
        https://github.com/nils-mathieu/ruel/issues/new\
        "
    );
    die();
}
