use sys::SysResult;

use crate::Result;

/// Represents a process.
///
/// # Remarks
///
/// This type does not guarantee that the process is still alive, or even whether the ID it
/// stores is actually valid.
#[derive(Debug, Clone, Copy)]
pub struct ProcessId(pub sys::ProcessId);

impl ProcessId {
    /// The special ID used to represent the current process.
    pub const SELF: Self = Self(sys::ProcessId::MAX);

    /// Attempts to despawns the process.
    ///
    /// See [`sys::despawn_process`] for more information.
    pub fn despawn(self) -> Result<()> {
        match sys::despawn_process(self.0) {
            SysResult::SUCCESS => Ok(()),
            err => Err(err),
        }
    }
}
