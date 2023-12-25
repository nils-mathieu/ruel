//! Defines the system call handlers.

use core::fmt::Write;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::sync::atomic::Ordering::Relaxed;

use ruel_sys::{Framebuffer, PciDevice, SysResult, Value, Verbosity, WakeUp};
use x86_64::{hlt, PageTableEntry, PhysAddr, VirtAddr};

use crate::cpu::paging::{MappingError, HHDM_OFFSET};
use crate::global::GlobalToken;
use crate::log;
use crate::process::{ProcessPtr, SleepingState};

/// Returns the provided value if the result is [`None`].
macro_rules! try_or {
    ($result:expr, $or:expr) => {
        match $result {
            Some(value) => value,
            None => return $or,
        }
    };
}

/// See [`ruel_sys::despawn_process`].
pub unsafe extern "C" fn despawn_process(
    process_id: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let glob = GlobalToken::get();

    let _process = try_or!(glob.processes.get(process_id), SysResult::PROCESS_NOT_FOUND);

    todo!("despawn_process({})", process_id);
}

/// See [`ruel_sys::sleep`].
pub unsafe extern "C" fn sleep(
    wake_ups: usize,
    wake_up_len: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    // FIXME: Properly ensure that the memory referenced is valid.

    // Put the current process to sleep.

    let glob = GlobalToken::get();

    {
        // Prevent interrupts while we're holding the lock.
        let mut current_process = glob.processes.current();

        // Convert the input pointers to the kernel's address space so that they can be accessed
        // even if the process ends up waking up in a different address space.
        let wake_up =
            current_process.address_space.translate(wake_ups).unwrap() as usize + HHDM_OFFSET;
        let wake_up = unsafe { NonNull::new_unchecked(wake_up as *mut WakeUp) };
        let wake_ups =
            unsafe { ProcessPtr::new(NonNull::slice_from_raw_parts(wake_up, wake_up_len)) };

        // Update the state of the process.
        assert!(current_process.sleeping.is_none());
        current_process.sleeping = Some(SleepingState { wake_ups });
    }

    // TODO: Switch to another process.
    // Currently, because we don't have multitasking, just halt until the process is woken up.

    while glob.processes.current().sleeping.is_some() {
        hlt();
    }

    SysResult::SUCCESS
}

/// See [`ruel_sys::acquire_framebuffers`].
pub unsafe extern "C" fn acquire_framebuffers(
    ret: usize,
    count: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let glob = GlobalToken::get();

    let ret = unsafe { (ret as *mut MaybeUninit<Framebuffer>).as_mut() };
    let count = unsafe { &mut *(count as *mut usize) };

    if glob.framebuffers.acquire(glob.processes.current_id()) {
        assert_eq!(glob.framebuffers.as_slice().len(), 1);

        if let Some(ret) = ret {
            let framebuffer = glob.framebuffers.as_slice()[0];

            // Allocate the framebuffer in the user's address space.
            let mut process = glob.processes.current();

            match process.address_space.map_range(
                0x100_0000, // TODO: Allocate virtual memory
                (framebuffer.address as VirtAddr - HHDM_OFFSET) as PhysAddr,
                framebuffer.size(),
                PageTableEntry::WRITABLE | PageTableEntry::USER_ACCESSIBLE,
            ) {
                Ok(()) => (),
                Err(MappingError::OutOfMemory) => return SysResult::OUT_OF_MEMORY,
                Err(MappingError::AlreadyMapped) => unreachable!("framebuffer already mapped"),
            }

            // Save the mapping in the metadata.
            let metadata = unsafe { glob.framebuffers.metadata_mut() };

            metadata[0].virt_address = framebuffer.address as usize;
            metadata[0].virt_size = framebuffer.size();

            ret.write(Framebuffer {
                address: 0x100_0000 as *mut u8,
                ..framebuffer
            });
        }

        *count = glob.framebuffers.as_slice().len();

        SysResult::SUCCESS
    } else {
        SysResult::RESOURCE_BUSY
    }
}

/// See [`ruel_sys::release_framebuffers`].
pub unsafe extern "C" fn release_framebuffers(
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let glob = GlobalToken::get();

    if glob.framebuffers.release(glob.processes.current_id()) {
        SysResult::SUCCESS
    } else {
        SysResult::MISSING_CAPABILITY
    }
}

/// See [`ruel_sys::read_value`].
pub unsafe extern "C" fn read_value(
    value: usize,
    result: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let glob = GlobalToken::get();

    match Value::from_raw(value) {
        Value::UPTICKS => {
            let result = unsafe { &mut *(result as *mut MaybeUninit<u64>) };
            result.write(glob.upticks.load(Relaxed));
        }
        Value::UPTIME => {
            let result = unsafe { &mut *(result as *mut MaybeUninit<ruel_sys::Duration>) };
            let ticks = glob.upticks.load(Relaxed);
            let ns_per_tick = crate::cpu::idt::pit::interval_ns();
            let total_ns = ticks as u128 * ns_per_tick as u128;
            let total_secs = (total_ns / 1_000_000_000) as u64;
            let subsec_ns = (total_ns % 1_000_000_000) as u64;
            result.write(ruel_sys::Duration {
                seconds: total_secs,
                nanoseconds: subsec_ns,
            });
        }
        Value::NANOSECONDS_PER_TICK => {
            let result = unsafe { &mut *(result as *mut MaybeUninit<u32>) };
            result.write(crate::cpu::idt::pit::interval_ns());
        }
        _ => return SysResult::INVALID_VALUE,
    }

    SysResult::SUCCESS
}

/// See [`ruel_sys::enumerate_pci_devices`].
pub unsafe extern "C" fn enumerate_pci_devices(
    devices: usize,
    count: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let glob = GlobalToken::get();

    let count = unsafe { &mut *(count as *mut usize) };

    unsafe {
        core::ptr::copy_nonoverlapping(
            glob.pci_devices.as_ptr(),
            devices as *mut PciDevice,
            glob.pci_devices.len().min(*count),
        );
    }

    *count = glob.pci_devices.len();

    SysResult::SUCCESS
}

/// See [`ruel_sys::kernel_log`].
pub unsafe extern "C" fn kernel_log(
    verbosity: usize,
    data: usize,
    len: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let verbosity = try_or!(Verbosity::from_raw(verbosity), SysResult::INVALID_VALUE);

    // FIXME: Make sure that the memory provided is valid.
    let data = unsafe { core::slice::from_raw_parts(data as *const u8, len) };

    struct ProcessMessage<'a> {
        data: &'a [u8],
    }

    impl<'a> core::fmt::Display for ProcessMessage<'a> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let mut slice = self.data;

            while !slice.is_empty() {
                match core::str::from_utf8(slice) {
                    Ok(s) => {
                        f.write_str(s)?;
                        break;
                    }
                    Err(error) => unsafe {
                        let valid = slice.get_unchecked(..error.valid_up_to());
                        f.write_str(core::str::from_utf8_unchecked(valid))?;
                        f.write_char(char::REPLACEMENT_CHARACTER)?;

                        if let Some(invalid_count) = error.error_len() {
                            slice = slice.get_unchecked(invalid_count..);
                        } else {
                            break;
                        }
                    },
                }
            }

            Ok(())
        }
    }

    log::log!(verbosity, "{}", ProcessMessage { data });

    SysResult::SUCCESS
}
