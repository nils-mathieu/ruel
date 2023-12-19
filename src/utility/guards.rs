use x86_64::{cli, sti, RFlags};

/// A guard that restores interrupts when dropped.
pub struct RestoreInterrupts;

impl RestoreInterrupts {
    /// Conditionally creates a new [`RestoreInterrupts`] instance if interrupts are currently
    /// enabled.
    ///
    /// If they are, this function clears the interrupt flag and returns a new
    /// [`RestoreInterrupts`] instance to restore them when dropped.
    pub fn without_interrupts() -> Option<Self> {
        if RFlags::read().intersects(RFlags::INTERRUPTS) {
            cli();
            Some(Self)
        } else {
            None
        }
    }
}

impl Drop for RestoreInterrupts {
    #[inline]
    fn drop(&mut self) {
        sti();
    }
}
