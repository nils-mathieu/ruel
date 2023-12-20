//! Provides simple syncronization primitives.

mod once_lock;
pub use self::once_lock::*;

mod mutex;
pub use self::mutex::*;

mod cpu_local;
pub use self::cpu_local::*;
