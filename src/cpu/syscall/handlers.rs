//! Defines the system call handlers.

use core::fmt::Write;

use ruel_sys::{ProcessId, Slice, SysResult, Verbosity};

use crate::log;

/// See [`ruel_sys::terminate`].
pub unsafe extern "C" fn terminate(
    process_id: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    if process_id == ProcessId::MAX {
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
