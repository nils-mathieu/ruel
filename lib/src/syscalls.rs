use core::arch::asm;

use crate::{
    Framebuffer, PS2Buffer, ProcessConfig, ProcessId, SysResult, Sysno, Verbosity, WakeUp,
};

/// Performs a system call with no arguments.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall0(no: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            lateout("rdi") _,
            lateout("rsi") _,
            lateout("rdx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Performs a system call with one argument.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall1(no: usize, a1: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            lateout("rsi") _,
            lateout("rdx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Performs a system call with two arguments.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall2(no: usize, a1: usize, a2: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            lateout("rdx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Performs a system call with three arguments.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall3(no: usize, a1: usize, a2: usize, a3: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Performs a system call with four arguments.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall4(no: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Performs a system call with five arguments.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall5(no: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            lateout("r9") _,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Performs a system call with six arguments.
///
/// # Safety
///
/// Some system calls can compromise the memory safety of the program.
#[inline]
pub unsafe fn syscall6(
    no: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            in("r9") a6,
            lateout("r11") _,
            lateout("r12") _,
        );
    }

    ret
}

/// Despawns (terminate) the specified process.
///
/// # Parameters
///
/// - `process_id`: The ID of the process to despawn. The special value `ProcessId::MAX` is used
///   to refer to the current process.
///
/// # Errors
///
/// - `PROCESS_NOT_FOUND` if the `process_id` does not refer to an existing process.
///
/// Note that using `ProcessId::MAX` as the `process_id` will never fail. In that case, the
/// function is guaranteed to never return control to the caller.
///
/// # Returns
///
/// Nothing; but this function diverges if `process_id` is `ProcessId::MAX` or the ID of the
/// current process.
#[inline]
pub fn despawn_process(process_id: ProcessId) -> SysResult {
    unsafe { SysResult::from_raw(syscall1(Sysno::DespawnProcess as usize, process_id)) }
}

/// Despawns (terminate) the current process.
///
/// # Returns
///
/// This function never returns.
#[inline]
pub fn despawn_self() -> ! {
    unsafe {
        let _ = despawn_process(ProcessId::MAX);
        debug_assert!(false, "despawn_self() returned");
        core::hint::unreachable_unchecked();
    }
}

/// Sets the configuration of the specified process.
///
/// # Parameters
///
/// - `process_id`: The ID of the process to configure. The special value `ProcessId::MAX` is used
///   to refer to the current process.
///
/// # Returns
///
/// - `PROCESS_NOT_FOUND` if the `process_id` does not refer to an existing process.
///
/// - `INVALID_VALUE` if any of the flags are invalid.
///
/// # Returns
///
/// Nothing.
///
/// # Safety
///
/// Some other part of the code my rely on the current configuration of the process.
#[inline]
pub unsafe fn set_process_config(process_id: ProcessId, flags: ProcessConfig) -> SysResult {
    unsafe {
        SysResult::from_raw(syscall2(
            Sysno::SetProcessConfig as usize,
            process_id,
            flags.bits(),
        ))
    }
}

/// Gets the configuration of the specified process.
///
/// # Parameters
///
/// - `process_id`: The ID of the process to configure. The special value `ProcessId::MAX` is used
///   to refer to the current process.
///
/// # Errors
///
/// - `PROCESS_NOT_FOUND` if the `process_id` does not refer to an existing process.
///
/// # Returns
///
/// - `ret`: A pointer to a [`ProcessConfig`] instance that will be filled with the configuration
///   of the process.
#[inline]
pub fn get_process_config(process_id: ProcessId, ret: *mut ProcessConfig) -> SysResult {
    unsafe {
        SysResult::from_raw(syscall2(
            Sysno::GetProcessConfig as usize,
            process_id,
            ret as usize,
        ))
    }
}

/// Puts the current process to sleep until it is woken up when any of the specified wake-up events
/// occur.
///
/// # Parameters
///
/// - `wake_ups`: A pointer to an array of [`WakeUp`] instances. This pointer must reference
///   at least `wake_up_len` items.
///
/// - `wake_up_len`: The number of items in the `wake_ups` array.
///
/// # Remarks
///
/// This function is unaffected by the [`DONT_BLOCK`] process configuration flag. Whathever the
/// value, the process will always be put to sleep.
///
/// # Errors
///
/// - `INVALID_VALUE` if any of the wake-up events are invalid.
///
/// # Returns
///
/// `index` is set to the index of the wake-up event that woke the process up.
///
/// When mutiple wake-up events occur at the same time, the index of the first one in the list
/// is returned.
///
/// [`DONT_BLOCK`]: ProcessConfig::DONT_BLOCK
#[inline]
pub fn sleep(wake_ups: *mut WakeUp, wake_up_len: usize) -> SysResult {
    unsafe {
        SysResult::from_raw(syscall2(
            Sysno::Sleep as usize,
            wake_ups as usize,
            wake_up_len,
        ))
    }
}

/// Reads the bytes received by the program on the first PS/2 port.
///
/// # Blocking Behavior
///
/// If the [`DONT_BLOCK`] process configuration flag is set, this system call will does not block
/// and returns instantly with an empty buffer if no bytes are available. Otherwise, the function
/// blocks until at least one byte of data is available.
///
/// # Returns
///
/// - `ret`: A pointer to a [`PS2Buffer`] instance that will be filled with the bytes received by
///   the program.
#[inline]
pub fn read_ps2(ret: *mut PS2Buffer) -> SysResult {
    unsafe { SysResult::from_raw(syscall1(Sysno::ReadPS2 as usize, ret as usize)) }
}

/// Acquires the framebuffers available on the system.
///
/// # Parameters
///
/// - `ret`: Either a null pointer, or a pointer to an array of [`Framebuffer`] instances.
///
/// - `count`: The maximum number of [`Framebuffer`] instances that can be written by the kernel
///   at `ret` (if non-null), and upon return, the number of framebuffers available on the system.
///
/// # Errors
///
/// - `RESOURCE_BUSY` if the framebuffers are currently owned by another process.
///
/// - `OUT_OF_MEMORY` if the kernel is unable to allocate memory for bookkeeping.
///
/// # Returns
///
/// At most `count` framebuffers are written to `ret`. If `count` is zero, `ret` is not observed.
///
/// The number of framebuffers available is written to `count`.
pub fn acquire_framebuffers(ret: *mut Framebuffer, count: *mut usize) -> SysResult {
    unsafe {
        SysResult::from_raw(syscall2(
            Sysno::AcquireFramebuffers as usize,
            ret as usize,
            count as usize,
        ))
    }
}

/// Releases the buffers available on the system.
///
/// # Errors
///
/// - `MISSING_CAPABILITY` if the current process does not own the framebuffers.
///
/// # Returns
///
/// Nothing.
pub fn release_framebuffers() -> SysResult {
    unsafe { SysResult::from_raw(syscall0(Sysno::ReleaseFramebuffers as usize)) }
}

/// Sends a message using the kernel's logging system.
///
/// # Parameters
///
/// - `verbosity`: The verbosity level of the message.
///
/// 0 is the lowest verbosity level (ERROR), then 1 (WARNING), 2 (INFO), and finally 3 (TRACE).
///
/// - `data`: A collection of [`Slice`]s containing the data to log. This pointer must reference
///   at least `data_len` items.
///
/// - `data_len`: The number of entries in the `data` array.
///
/// # Errors
///
/// - `INVALID_VALUE` if `verbosity` is not in the range `0..=3`. This cannot happen with the API
///   provided by this crate as it uses a Rust enumeration that is guaranteed to be in that range.
///
/// # Returns
///
/// Nothing.
#[inline]
pub fn kernel_log(verbosity: Verbosity, data: *const u8, data_len: usize) -> SysResult {
    unsafe {
        SysResult::from_raw(syscall3(
            Sysno::KernelLog as usize,
            verbosity as usize,
            data as usize,
            data_len,
        ))
    }
}
