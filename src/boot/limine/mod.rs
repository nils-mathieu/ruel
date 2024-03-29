//! Defines the entry point of the kernel when it is booted by a Limine-compliant bootloader.
//!
//! This is a simple implementation of the [Limine boot protocol][PROTOCOL].
//!
//! [PROTOCOL]: https://github.com/limine-bootloader/limine/blob/v6.x-branch/PROTOCOL.md
//!
//! # Version
//!
//! The version 6 of the protocol is implemented.

use core::alloc::Layout;
use core::arch::asm;
use core::mem::size_of;
use core::sync::atomic::AtomicU64;

use limine::{File, FramebufferMemoryModel, MemmapEntry, MemmapType};
use ruel_sys::{Framebuffer, FramebufferFormat};
use x86_64::{sti, Efer, PageTable, PageTableEntry, PhysAddr, VirtAddr};

use crate::boot::{handle_mapping_error, oom};
use crate::cpu::paging::{
    AddressSpace, AddressSpaceContext, HhdmToken, FOUR_KIB, HHDM_OFFSET, KERNEL_BIT, NOT_OWNED_BIT,
};
use crate::global::{Framebuffers, Global, MemoryAllocator, OutOfMemory, Processes};
use crate::hcf::die;
use crate::log;
use crate::process::Registers;
use crate::sync::Mutex;
use crate::utility::array_vec::ArrayVec;
use crate::utility::{BumpAllocator, HumanByteCount, PhysBumpAllocator};

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

    if !limine::base_revision_supported() {
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

    let mut usable_memory = ArrayVec::new_array();
    validate_and_find_usable_segments(memory_map, &mut usable_memory);

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

    log::trace!("Looking for the init program...");

    let init_program = unsafe { find_init_program(token.modules()) };

    if !init_program.media_type.is_known() {
        log::warn!(
            "\
            The init program is stored on a media type that is not known to the kernel.\n\
            This is not necessarily a bug in the bootloader; but it is pretty weird.\
            "
        );
    }

    log::trace!(
        "\
        Found the init program:\n\
        > Path    = `{}`\n\
        > Cmdline = `{}`\n\
        > Size    = {}\n\
        > Media   = {:?}\
        ",
        unsafe { init_program.path.as_cstr().to_bytes().escape_ascii() },
        unsafe { init_program.cmdline.as_cstr().to_bytes().escape_ascii() },
        HumanByteCount(init_program.size),
        init_program.media_type,
    );

    // Save the init program's physical address so that we can load it later on (even if the
    // HHDM changes).
    let init_program_phys_addr = init_program.address.as_ptr() as u64 - bootloader_hhdm;

    let mut usable_framebuffers = ArrayVec::new_array();
    parse_framebuffers(
        token.framebuffer(),
        bootloader_hhdm as usize,
        &mut usable_framebuffers,
    );

    // =============================================================================================
    // Bootstrap Allocator
    // =============================================================================================
    log::trace!("Finding a suitable block for the bootstrap allocator...");

    let largest_usable_segment = find_largest_usable_segment(memory_map);

    log::trace!(
        "The bootstrap allocator will use the memory segment {:#x}..{:#x} ({})",
        largest_usable_segment.base,
        largest_usable_segment.base + largest_usable_segment.length,
        HumanByteCount(largest_usable_segment.length),
    );

    // SAFETY:
    //  The ownership of the largest usable segment is transferred to the bootstrap allocator. We
    //  won't be accessing it until the allocator is no longer used.
    let mut bootstrap_allocator = unsafe {
        PhysBumpAllocator::new(
            largest_usable_segment.base,
            largest_usable_segment.base + largest_usable_segment.length,
        )
    };

    // Save the init program's command-line arguments so that we can use them later, even when
    // the bootloader reclaimable memory region is no longer available.
    let init_program_cmdline = unsafe { init_program.cmdline.as_cstr().to_bytes() };

    let init_program_cmdline_phys_addr = bootstrap_allocator
        .allocate(Layout::for_value(init_program_cmdline))
        .unwrap_or_else(|_| oom());

    unsafe {
        core::ptr::copy_nonoverlapping(
            init_program_cmdline.as_ptr(),
            (init_program_cmdline_phys_addr + bootloader_hhdm) as *mut u8,
            init_program_cmdline.len(),
        );
    }

    // =============================================================================================
    // Address Space & Kernel Stack
    // =============================================================================================
    log::trace!("Creating the kernel address space...");

    // Make sure that the NO_EXECUTE bit on pages is available.
    Efer::read().union(Efer::NO_EXECUTE).write();

    // Create the kernel's address space.
    let address_space =
        unsafe {
            create_kernel_address_space(
                &mut bootstrap_allocator,
                bootloader_hhdm as usize,
                kernel_address.physical_base,
                find_memory_upper_bound(memory_map),
            )
        };

    log::trace!("Kernel L4 Table stored at address {:#x}", address_space);

    // Allocate the kernel stack.
    const KERNEL_STACK_SIZE: usize = 16 * FOUR_KIB;
    let kernel_stack_base = bootstrap_allocator
        .allocate(Layout::new::<[u8; KERNEL_STACK_SIZE]>())
        .unwrap_or_else(|_| oom());
    let kernel_stack_top = kernel_stack_base as usize + HHDM_OFFSET + KERNEL_STACK_SIZE;

    log::trace!("Kernel stack allocated at address: {:#x}", kernel_stack_top);

    // Allocate the `ToNewStack` instance that will be passed to the new stack.
    let to_new_stack_phys_addr = bootstrap_allocator
        .allocate(Layout::new::<ToNewStack>())
        .unwrap_or_else(|_| oom());

    unsafe {
        core::ptr::write(
            (to_new_stack_phys_addr + bootloader_hhdm) as *mut ToNewStack,
            ToNewStack {
                bootstrap_allocator,
                kernel_stack_top,
                usable_framebuffers,
                usable_memory,
                kernel_physical_base: kernel_address.physical_base,
                init_process: core::slice::from_raw_parts(
                    (init_program_phys_addr as usize + HHDM_OFFSET) as *const u8,
                    init_program.size as usize,
                ),
                init_process_cmdline: core::slice::from_raw_parts(
                    (init_program_cmdline_phys_addr as usize + HHDM_OFFSET) as *const u8,
                    init_program_cmdline.len(),
                ),
                address_space,
            },
        );
    }

    log::trace!("Switching address space...");

    unsafe {
        asm!(
            "
            mov cr3, {l4_table}
            mov rsp, {new_stack}
            mov rbp, {new_stack}
            call {with_new_stack}
            ",
            l4_table = in(reg) address_space,
            new_stack = in(reg) kernel_stack_top,
            in("rdi") to_new_stack_phys_addr as usize + HHDM_OFFSET,
            with_new_stack = sym with_new_stack,
            options(noreturn, preserves_flags)
        );
    }
}

/// A structure that's passed from the bootloader's stack to the kernel's stack.
///
/// Because virtual-memory references are invalidated, we need to copy everything we need
/// from the bootloader's stack to the kernel's stack (or save their physical addresses).
struct ToNewStack {
    /// The allocator that's being used to allocate memory during the booting process.
    bootstrap_allocator: PhysBumpAllocator,
    /// The virtual address of the kernel stack.
    kernel_stack_top: VirtAddr,
    /// The segments that are usable by the global allocator.
    ///
    /// # Remarks
    ///
    /// Those segments do include the segment that is currently used by the bootstrap
    /// allocator. We need to be careful not to mark the pages it has already issued as free.
    usable_memory: ArrayVec<MemmapEntry, 8>,
    /// The usable framebuffers.
    usable_framebuffers: ArrayVec<Framebuffer, 4>,
    /// The physical address of the kernel image.
    kernel_physical_base: PhysAddr,

    /// The init process.
    init_process: &'static [u8],
    /// The command-line arguments of the init process.
    init_process_cmdline: &'static [u8],

    /// The physical address of the kernel's L4 page table.
    address_space: PhysAddr,
}

/// The function that is called upon entering the new stack and address space.
extern "C" fn with_new_stack(package: *mut ToNewStack) -> ! {
    let ToNewStack {
        bootstrap_allocator,
        kernel_stack_top,
        usable_memory,
        kernel_physical_base,
        init_process,
        init_process_cmdline,
        address_space,
        usable_framebuffers,
    } = unsafe { package.read() };

    // SAFETY:
    //  The HHDM has been initiated when we changed address-space.
    let hhdm = unsafe { HhdmToken::get() };
    let mut bootstrap_allocator = BumpAllocator::new(bootstrap_allocator, hhdm);

    // =============================================================================================
    // CPU Initialization
    // =============================================================================================
    crate::cpu::gdt::init(&mut bootstrap_allocator, kernel_stack_top).unwrap_or_else(|_| oom());
    crate::cpu::idt::init(&mut bootstrap_allocator).unwrap_or_else(|_| oom());
    let pci_devices = crate::io::pci::init(&mut bootstrap_allocator).unwrap_or_else(|_| oom());

    // =============================================================================================
    // Global Kernel State
    // =============================================================================================
    let processes = Processes::new(&mut bootstrap_allocator).unwrap_or_else(|_| oom());
    let allocator = initialize_global_allocator(&usable_memory, bootstrap_allocator, hhdm);

    log::trace!("Initializing the global kernel state...");
    let glob = crate::global::init(
        Global {
            allocator: Mutex::new(allocator),
            kernel_physical_base,
            address_space,
            processes,
            framebuffers: Framebuffers::new(usable_framebuffers),
            upticks: AtomicU64::new(0),
            pci_devices,
        },
        kernel_stack_top,
    );

    // =============================================================================================
    // System Calls
    // =============================================================================================
    crate::cpu::syscall::init();

    // =============================================================================================
    // Init Program Loading
    // =============================================================================================
    let id = glob
        .processes
        .spawn_process(crate::boot::init_process::load_any(init_process, init_process_cmdline))
        .unwrap();
    glob.processes.schedule(id).unwrap();

    // Allow interrupts.
    sti();

    log::info!("Spawning the init process!");

    unsafe {
        let (l4_table, registers) = {
            let current = glob.processes.current();
            (current.address_space.l4_table(), current.registers)
        };

        asm!(
            "
            mov cr3, {address_space}
            mov rcx, [r11 + 8 * {RIP_INDEX}]
            mov rsp, [r11 + 8 * {RSP_INDEX}]
            mov rbp, [r11 + 8 * {RBP_INDEX}]
            mov rdi, [r11 + 8 * {RDI_INDEX}]
            mov r11, 0x202
            sysretq
            ",
            in("r11") &registers,
            address_space = in(reg) l4_table,
            RIP_INDEX = const Registers::RIP_INDEX,
            RSP_INDEX = const Registers::RSP_INDEX,
            RBP_INDEX = const Registers::RBP_INDEX,
            RDI_INDEX = const Registers::RDI_INDEX,
            options(noreturn)
        );
    }
}

/// Parses the framebuffer information provided by the bootloader.
fn parse_framebuffer(framebuffer: &limine::Framebuffer0, hhdm: usize) -> Option<Framebuffer> {
    let format = match (
        framebuffer.memory_model,
        framebuffer.bpp,
        framebuffer.red_mask_size,
        framebuffer.red_mask_shift,
        framebuffer.green_mask_size,
        framebuffer.green_mask_shift,
        framebuffer.blue_mask_size,
        framebuffer.blue_mask_shift,
    ) {
        (FramebufferMemoryModel::RGB, 32, 8, 16, 8, 8, 8, 0) => FramebufferFormat::BGR32,
        (FramebufferMemoryModel::RGB, 32, 8, 0, 8, 8, 8, 16) => FramebufferFormat::RGB32,
        (FramebufferMemoryModel::RGB, 24, 8, 16, 8, 8, 8, 0) => FramebufferFormat::BGR24,
        (FramebufferMemoryModel::RGB, 24, 8, 0, 8, 8, 8, 16) => FramebufferFormat::RGB24,
        _ => {
            log::warn!(
                "\
                Framebuffer not supported: the format of the framebuffer is\n\
                not supported by the kernel.\n\
                \n\
                > Memory model: {:?}\n\
                > Bits per pixel: {}\n\
                > Red: {}..{}\n\
                > Green: {}..{}\n\
                > Blue: {}..{}\n\
                ",
                framebuffer.memory_model,
                framebuffer.bpp,
                framebuffer.red_mask_shift,
                framebuffer.red_mask_shift + framebuffer.red_mask_size,
                framebuffer.green_mask_shift,
                framebuffer.green_mask_shift + framebuffer.green_mask_size,
                framebuffer.blue_mask_shift,
                framebuffer.blue_mask_shift + framebuffer.blue_mask_size,
            );
            return None;
        }
    };

    if framebuffer.width > u32::MAX as u64 || framebuffer.height > u32::MAX as u64 {
        log::warn!(
            "\
            Framebuffer not supported: the framebuffer is too large.\n\
            \n\
            > Width: {}\n\
            > Height: {}\n\
            ",
            framebuffer.width,
            framebuffer.height,
        );
        return None;
    }

    Some(Framebuffer {
        bytes_per_lines: framebuffer.pitch as usize,
        format,
        address: (framebuffer.address as usize - hhdm + HHDM_OFFSET) as *mut u8,
        width: framebuffer.width as u32,
        height: framebuffer.height as u32,
    })
}

/// Parses the framebuffers provided by the bootloader.
fn parse_framebuffers(
    framebuffer: &[&limine::Framebuffer0],
    hhdm: usize,
    out: &mut ArrayVec<Framebuffer, 4>,
) {
    log::trace!("Parsing the framebuffers provided by the bootloader...");

    let mut dropped_framebuffers = false;

    for framebuffer in framebuffer {
        if let Some(framebuffer) = parse_framebuffer(framebuffer, hhdm) {
            if out.try_push(framebuffer).is_err() {
                dropped_framebuffers = true;
                break;
            }
        }
    }

    if dropped_framebuffers {
        log::warn!(
            "\
            The kernel is unable to handle more than {} framebuffers.\n\
            The kernel will continue to boot, but it will not be able to use all\n\
            of the available framebuffers.\n\
            \n\
            If this is a problem for you, please file an issue on the GitHub
            repository!\n\
            \n\
            https://github.com/nils-mathieu/ruel/issues/new\
            ",
            out.capacity(),
        );
    }

    log::trace!("Found {} supported framebuffers.", out.len());
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
fn validate_and_find_usable_segments(
    memory_map: &[&MemmapEntry],
    usable_memory: &mut ArrayVec<MemmapEntry, 8>,
) {
    let mut last_entry: Option<&MemmapEntry> = None;
    let mut too_many_segments = false;
    let mut total_usable_memory = 0;

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

            if let Some(last_usable) = usable_memory.last_mut() {
                if last_usable.base + last_usable.length == entry.base {
                    last_usable.length += entry.length;
                } else {
                    too_many_segments |= usable_memory.try_push(**entry).is_err();
                }
            } else {
                too_many_segments |= usable_memory.try_push(**entry).is_err();
            }

            total_usable_memory += entry.length;
        }

        last_entry = Some(entry);
    }

    if too_many_segments {
        let deteced_memory = usable_memory.iter().map(|entry| entry.length).sum::<u64>();

        log::warn!(
            "\
            Seems like the memory on your system is particularly fragmented.\n\
            Due to the laziness of the kernel's developers, the kenrel is unable\n\
            to handle more than {} usable memory segments.\n\
            \n\
            The kernel will continue to boot, but it will not be able to use all\n\
            of the available memory.\n\
            \n\
            Available memory: {}\n\
            Memory taken in account: {}\n\
            \n\
            If this is a problem for you, please file an issue on the GitHub
            repository!\n\
            \n\
            https://github.com/nils-mathieu/ruel/issues/new\
            ",
            usable_memory.capacity(),
            HumanByteCount(total_usable_memory),
            HumanByteCount(deteced_memory),
        );
    } else {
        log::info!("Available memory: {}", HumanByteCount(total_usable_memory));
    }
}

/// Initializes the global allocator.
///
/// # Remarks
///
/// This function takes ownership of the bootstrap allocator because after this function has been
/// called, the bootstrap allocator should no longer be used. The ownership of the remaining
/// pages is transferred to the global allocator.
fn initialize_global_allocator(
    usable_memory: &[MemmapEntry],
    mut bootstrap_allocator: BumpAllocator,
    hhdm: HhdmToken,
) -> MemoryAllocator {
    log::trace!("Initializing the global allocator...");

    let pages_needed = usable_memory
        .iter()
        .map(|e| e.length as usize / FOUR_KIB)
        .sum::<usize>();

    log::trace!(
        "The global allocator will need {} pages to store the free list ({}).",
        pages_needed,
        HumanByteCount(pages_needed as u64 * size_of::<PhysAddr>() as u64),
    );

    let mut allocator = unsafe {
        MemoryAllocator::empty(hhdm, &mut bootstrap_allocator, pages_needed)
            .unwrap_or_else(|_| oom())
    };

    // Compute the range of pages that have been used by the bootstrap allocator to avoid
    // pushing them to the free list later on.
    let used_start = x86_64::page_align_down(bootstrap_allocator.inner.top() as usize) as PhysAddr;
    let used_stop = bootstrap_allocator.inner.original_top();

    log::trace!(
        "Total memory used during boot: {}",
        HumanByteCount(used_stop - used_start)
    );

    for entry in usable_memory {
        let mut base = entry.base;
        let mut length = entry.length;

        while length != 0 {
            if base < used_start || base >= used_stop {
                unsafe { allocator.assume_available(base) };
            }

            base += FOUR_KIB as u64;
            length -= FOUR_KIB as u64;
        }
    }

    allocator
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
    boostrap_allocator: &mut PhysBumpAllocator,
    hhdm: usize,
    kernel_physical_base: PhysAddr,
    memory_upper_bound: PhysAddr,
) -> PhysAddr {
    struct Context<'a> {
        allocator: &'a mut PhysBumpAllocator,
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

    let mut address_space =
        AddressSpace::new(Context {
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
            PageTableEntry::WRITABLE | PageTableEntry::GLOBAL | NOT_OWNED_BIT | KERNEL_BIT,
        )
        .unwrap_or_else(|err| handle_mapping_error(err));

    let start_page = x86_64::page_align_down(crate::linker::kernel_image_begin() as VirtAddr);
    let stop_page = x86_64::page_align_up(crate::linker::kernel_image_end() as VirtAddr);

    // Map the kernel's physical memory into the address space, at the position that's specified
    // in the linker script.
    address_space
        .map_range(
            start_page,
            x86_64::page_align_down(kernel_physical_base as usize) as PhysAddr,
            stop_page - start_page,
            PageTableEntry::WRITABLE | PageTableEntry::GLOBAL | NOT_OWNED_BIT | KERNEL_BIT,
        )
        .unwrap_or_else(|err| handle_mapping_error(err));

    address_space.leak()
}

/// Finds the init program in the provided modules.
///
/// # Safety
///
/// The memory referenced by the files must still be around.
unsafe fn find_init_program<'a>(modules: &[&'a File]) -> &'a File {
    let mut found = None;

    for module in modules {
        let name = basename(unsafe { module.path.as_cstr().to_bytes() });

        if name == b"alibert" {
            if found.is_none() {
                found = Some(module);
            } else {
                log::warn!(
                    "Found duplicate module: `alibert` ({})",
                    name.escape_ascii(),
                );
            }
        } else {
            log::warn!("Unknown module: `{}`", name.escape_ascii());
        }
    }

    found.unwrap_or_else(|| {
        log::error!(
            "\
            The init program could not be found in the modules provided by the bootloader.\n\
            The kernel is unable to continue without an init program.\n\
            \n\
            The kernel expects a module named 'alibert' to be provided by the bootloader.\n\
            Try adding the following lines to your `limine.cfg` file:\n\
            \n\
            MODULE_PATH=boot:///some_path/alibert\n\
            MODULE_CMDLINE=optional command line arguments\
            ",
        );
        die();
    })
}

/// Returns the basename of the provided path.
fn basename(path: &[u8]) -> &[u8] {
    path.iter()
        .rposition(|&c| c == b'/')
        .map_or(path, |idx| &path[idx + 1..])
}
