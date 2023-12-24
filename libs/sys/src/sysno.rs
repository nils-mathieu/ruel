/// A list of all the system calls supported by the kernel.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sysno {
    /// See [`despawn_process`](crate::despawn_process).
    DespawnProcess,
    /// See [`sleep`](crate::sleep).
    Sleep,
    /// See [`acquire_framebuffers`](crate::acquire_framebuffers).
    AcquireFramebuffers,
    /// See [`release_framebuffers`](crate::release_framebuffers).
    ReleaseFramebuffers,
    /// See [`read_value`](crate::read_value).
    ReadValue,
    /// See [`kernel_log`](crate::kernel_log).
    KernelLog,
}
