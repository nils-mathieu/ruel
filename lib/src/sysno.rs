/// A list of all the system calls supported by the kernel.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sysno {
    /// See [`despawn_process`](crate::despawn_process).
    DespawnProcess,
    /// See [`set_process_config`](crate::set_process_config).
    SetProcessConfig,
    /// See [`get_process_config`](crate::get_process_config).
    GetProcessConfig,
    /// See [`sleep`](crate::sleep).
    Sleep,
    /// See [`read_ps2`](crate::read_ps2).
    ReadPS2,
    /// See [`kernel_log`](crate::kernel_log).
    KernelLog,
}
