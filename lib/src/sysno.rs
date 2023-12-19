/// A list of all the system calls supported by the kernel.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sysno {
    /// See [`terminate`](crate::terminate).
    Terminate,
    /// See [`kernel_log`](crate::kernel_log).
    KernelLog,
}
