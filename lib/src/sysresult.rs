/// The return-type of system calls.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[must_use = "this `SysResult` might contain an error that should be handled"]
pub struct SysResult(usize);

impl SysResult {
    /// Creates a new [`SysResult`] from the provided raw value.
    #[inline]
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    /// Returns the raw value of this [`SysResult`].
    #[inline]
    pub const fn as_raw(self) -> usize {
        self.0
    }

    /// Panics if this [`SysResult`] represents an error.
    #[inline]
    #[track_caller]
    pub fn unwrap(self) {
        assert_eq!(self, Self::SUCCESS, "called `.unwrap()` on an error");
    }
}

macro_rules! define_error_codes {
    (
        $(
            $(#[$($attr:meta)*])*
            $desc:literal
            const $code:ident = $value:expr;
        )*
    ) => {
        impl SysResult {
            $(
                $(#[$($attr)*])*
                pub const $code: Self = Self($value);
            )*

            /// Returns a short description of the error.
            pub const fn description(self) -> &'static str {
                match self {
                    $(
                        Self::$code => $desc,
                    )*
                    _ => "unknown SysResult",
                }
            }
        }

        impl core::fmt::Debug for SysResult {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match *self {
                    $(
                        Self::$code => f.write_str($desc),
                    )*
                    _ => f.debug_tuple("SysResult").field(&self.0).finish(),
                }
            }
        }
    };
}

define_error_codes! {
    /// The operation succeeded.
    "success"
    const SUCCESS = 0;

    /// This error can be returned under many circumstances, and indicate that the operation failed
    /// because one of the arguments passed to the system call was invalid.
    ///
    /// Refer to the documentation of the specific system call for more information.
    "invalid value"
    const INVALID_VALUE = 1;

    /// A process was used as an argument to a system call, but that process was not found
    /// (i.e. it does not exist or it has exited).
    "process not found"
    const PROCESS_NOT_FOUND = 2;
}
