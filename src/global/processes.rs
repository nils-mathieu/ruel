use core::cell::Cell;

use ruel_sys::{ProcessId, WakeUpTag};

use super::{Inputs, OutOfMemory};
use crate::process::Process;
use crate::sync::{CpuLocal, Mutex, MutexGuard};
use crate::utility::{BumpAllocator, StableFixedVec};

/// An error that's returned when a process could not be found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessNotFound;

/// An error that's returned when too many processes are running on the system and one cannot
/// be spawned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyProcesses;

/// The collection of all processes in the system.
pub struct Processes {
    /// The processes that are currently running on the system.
    list: Mutex<StableFixedVec<Process>>,
    /// The index of the current process running on each CPU.
    ///
    /// The special value `ProcessId::MAX` means that no process is currently running on the CPU.
    current_process: CpuLocal<Cell<ProcessId>>,
}

impl Processes {
    /// Creates a new empty [`Processes`] collection.
    pub fn new(boostrap_allocator: &mut BumpAllocator) -> Result<Self, OutOfMemory> {
        Ok(Self {
            list: Mutex::new(StableFixedVec::new(boostrap_allocator, 1024)?),
            current_process: CpuLocal::new(boostrap_allocator)?,
        })
    }

    /// Returns whether the given process ID is currently present in the system.
    #[inline]
    pub fn exists(&self, id: ProcessId) -> bool {
        self.list.lock().is_present(id)
    }

    /// Schedules the given process to run on the current CPU.
    ///
    /// # Safety
    ///
    /// The provided process ID must be valid.
    #[inline]
    pub fn schedule_unchecked(&self, id: ProcessId) {
        self.current_process.set(id);
    }

    /// Schedules the given process to run on the current CPU.
    pub fn schedule(&self, id: ProcessId) -> Result<(), ProcessNotFound> {
        if self.exists(id) {
            self.schedule_unchecked(id);
            Ok(())
        } else {
            Err(ProcessNotFound)
        }
    }

    /// Attempts to spawn a process on the system.
    pub fn spawn_process(&self, process: Process) -> Result<ProcessId, TooManyProcesses> {
        let mut list = self.list.lock();
        list.push(process).map_err(|_| TooManyProcesses)
    }

    /// Returns the process ID of the process currently running on the CPU.
    #[inline]
    pub fn current_id(&self) -> ProcessId {
        self.current_process.get()
    }

    /// Returns the current process.
    ///
    /// # Panics
    ///
    /// This function panics if no process is running on the current CPU.
    pub fn current(&self) -> MutexGuard<Process> {
        unsafe {
            let index = self.current_id();

            assert!(
                index != ProcessId::MAX,
                "Attempted to access the current process while no process is running on the CPU"
            );

            MutexGuard::map(self.list.lock(), |list| {
                debug_assert!(list.is_present(index));
                list.get_unchecked_mut(index)
            })
        }
    }

    /// Ticks the whole list of processes, checking if some can wake up.
    pub fn tick(&self, inputs: &mut Inputs) {
        let mut list = self.list.lock();

        for process in list.iter_mut() {
            let mut woken_up = false;

            if let Some(sleeping) = &mut process.sleeping {
                for wake_up in unsafe { sleeping.wake_ups.as_mut() } {
                    match wake_up.tag() {
                        WakeUpTag::PS2_KEYBOARD => {
                            let wake_up = unsafe { &mut wake_up.ps2_keyboard };
                            if !inputs.ps2_keyboard.is_empty() {
                                wake_up.count = inputs.ps2_keyboard.len() as u8;
                                unsafe {
                                    core::ptr::copy_nonoverlapping(
                                        inputs.ps2_keyboard.as_ptr(),
                                        wake_up.data.as_mut_ptr(),
                                        wake_up.count as usize,
                                    );
                                }
                                woken_up = true;
                            }
                        }
                        tag => unreachable!("invalid WakeUpTag detected: {tag:?}"),
                    }
                }
            }

            if woken_up {
                process.sleeping = None;
            }
        }

        inputs.clear();
    }
}
