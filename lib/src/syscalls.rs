use core::arch::asm;

use crate::{ProcessId, Slice, SysResult, Sysno, Verbosity, WakeUp};

/// Performs a system call with no arguments.
#[inline]
pub fn syscall0(no: usize) -> usize {
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

/// Terminates the specified process.
///
/// # Parameters
///
/// - `process_id`: The ID of the process to terminate. The special value `ProcessId::MAX` is used
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
pub fn terminate(process_id: ProcessId) -> SysResult {
    unsafe { SysResult::from_raw(syscall1(Sysno::Terminate as usize, process_id)) }
}

/// Terminates the current process.
///
/// # Returns
///
/// This function never returns.
#[inline]
pub fn terminate_self() -> ! {
    unsafe {
        let _ = terminate(ProcessId::MAX);
        debug_assert!(false, "terminate_self() returned");
        core::hint::unreachable_unchecked();
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
/// # Returns
///
/// - `INVALID_VALUE` if any of the wake-up events are invalid.
///
/// # Returns
///
/// `index` is set to the index of the wake-up event that woke the process up.
///
/// When mutiple wake-up events occur at the same time, the index of the first one in the list
/// is returned.
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
pub fn kernel_log(verbosity: Verbosity, data: *const Slice, data_len: usize) -> SysResult {
    unsafe {
        SysResult::from_raw(syscall3(
            Sysno::KernelLog as usize,
            verbosity as usize,
            data as usize,
            data_len,
        ))
    }
}
