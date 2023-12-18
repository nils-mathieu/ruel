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
use crate::hcf::die;
use crate::log;
use crate::mem::BumpAllocator;

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

    // =============================================================================================
    // Initial Allocator
    // =============================================================================================
    log::trace!("Finding a suitable block for the bootstrap allocator...");

    let largest_usable_segment = find_largest_usable_segment(memory_map).unwrap();

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

    let _ = bootstrap_allocator.allocate(core::alloc::Layout::new::<()>());

    todo!();
}

/// Finds the largest usable memory segment in the memory map.
fn find_largest_usable_segment<'a>(memory_map: &[&'a MemmapEntry]) -> Option<&'a MemmapEntry> {
    memory_map
        .iter()
        .filter(|entry| entry.ty == MemmapType::USABLE)
        .max_by_key(|entry| entry.length)
        .copied()
}

/// Validates the memory map provided by the bootloader.
///
/// If the map is found to break some of the invariants specified in the protocol, the function
/// stops the CPU.
fn validate_memory_map(memory_map: &[&MemmapEntry]) {
    let mut last_entry: Option<&MemmapEntry> = None;

    for entry in memory_map {
        match &mut last_entry {
            Some(last_entry) => {
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
            None => last_entry = Some(*entry),
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
    }
}
