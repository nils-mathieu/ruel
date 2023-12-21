use core::cell::Cell;

use ruel_sys::ProcessId;

use super::OutOfMemory;
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
        let list = self.list.lock();
        if list.is_present(id) {
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
    ///
    /// # Panics
    ///
    /// This function panics if no process is running on the current CPU.
    #[inline]
    pub fn current_id(&self) -> ProcessId {
        let id = self.current_process.get();

        assert!(
            id != ProcessId::MAX,
            "Attempted to access the current process while no process is running on the CPU"
        );

        id
    }

    /// Returns the current process.
    ///
    /// # Panics
    ///
    /// This function panics if no process is running on the current CPU.
    pub fn current(&self) -> MutexGuard<Process> {
        let id = self.current_id();
        MutexGuard::map(self.list.lock(), move |list| {
            debug_assert!(list.is_present(id));
            unsafe { list.get_unchecked_mut(id) }
        })
    }

    /// Attempts to get the process with the given ID.
    ///
    /// The special value `ProcessId::MAX` is used to refer to the current process.
    pub fn get(&self, id: ProcessId) -> Option<MutexGuard<Process>> {
        MutexGuard::try_map(self.list.lock(), |list| {
            if id == ProcessId::MAX {
                let id = self.current_id();
                debug_assert!(list.is_present(id));
                Ok(unsafe { list.get_unchecked_mut(id) })
            } else {
                list.get_mut(id).ok_or(())
            }
        })
        .ok()
    }

    /// Calls the provided closure with a reference to each process.
    #[inline]
    pub fn for_each_mut(&self, f: impl FnMut(&mut Process)) {
        self.list.lock().iter_mut().for_each(f)
    }
}
