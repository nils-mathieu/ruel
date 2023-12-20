//! Defines the system call handlers.

use core::fmt::Write;
use core::ptr::NonNull;

use ruel_sys::{ProcessId, Slice, SysResult, Verbosity, WakeUp};
use x86_64::hlt;

use crate::cpu::paging::HHDM_OFFSET;
use crate::global::GlobalToken;
use crate::log;
use crate::process::{ProcessPtr, SleepingState};
use crate::utility::RestoreInterrupts;

/// See [`ruel_sys::terminate`].
pub unsafe extern "C" fn terminate(
    process_id: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let glob = GlobalToken::get();

    if process_id == ProcessId::MAX || process_id == glob.processes.current_id() {
        todo!("terminate_self()");
    }

    SysResult::PROCESS_NOT_FOUND
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
    let verbosity = match Verbosity::from_raw(verbosity) {
        Some(verbosity) => verbosity,
        None => return SysResult::INVALID_VALUE,
    };

    // FIXME: Make sure that the memory provided is valid.
    let data = unsafe { core::slice::from_raw_parts(data as *const Slice, len) };

    struct ProcessMessage<'a> {
        data: &'a [Slice],
    }

    impl<'a> core::fmt::Display for ProcessMessage<'a> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            for slice in self.data {
                let mut slice = unsafe { core::slice::from_raw_parts(slice.address, slice.length) };

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
            }

            Ok(())
        }
    }

    log::log!(verbosity, "{}", ProcessMessage { data });

    SysResult::SUCCESS
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

    {
        let wake_ups = unsafe {
            core::slice::from_raw_parts(wake_ups as *const ruel_sys::WakeUp, wake_up_len)
        };

        // Ensure that the wake-up events are properly constructed.

        for wake_up in wake_ups {
            if !wake_up.tag().is_known() {
                return SysResult::INVALID_VALUE;
            }
        }
    }

    // Put the current process to sleep.

    let glob = GlobalToken::get();

    {
        // Prevent interrupts while we're holding the lock.
        let _without_interrupts = RestoreInterrupts::without_interrupts();

        let mut current_process = glob.processes.current();

        // Convert the input pointers to the kernel's address space so that they can be accessed
        // even if the process ends up waking up in a different address space.
        // FIXME: Properly handle errors.
        let wake_up =
            current_process.address_space.translate(wake_ups).unwrap() as usize + HHDM_OFFSET;
        let wake_up = unsafe { NonNull::new_unchecked(wake_up as *mut WakeUp) };
        let wake_ups = ProcessPtr::new(NonNull::slice_from_raw_parts(wake_up, wake_up_len));

        // Update the state of the process.
        assert!(current_process.sleeping.is_none());
        current_process.sleeping = Some(SleepingState { wake_ups });
    }

    // TODO: Switch to another process.
    // Currently, because we don't have multitasking, just halt until the process is woken up.

    loop {
        if glob.processes.current().sleeping.is_none() {
            break;
        }

        hlt();
    }

    SysResult::SUCCESS
}
