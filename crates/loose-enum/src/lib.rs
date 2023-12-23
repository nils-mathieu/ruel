#![no_std]

/// Creates a type that acts like an enum, but internally allows every bit patterns (unknown
/// values). This makes the library safer than using a regular enum, as it prevents
/// undefined behavior in case the bootloader sends an unknown value for some reason (for example
/// because it uses a version that we do not support).
///
/// The syntax is basically the same as the [`bitflags!`] macro.
#[macro_export]
macro_rules! loose_enum {
    (
        $(#[$($attr:meta)*])*
        $vis:vis struct $name:ident: $inner:ty {
            $(
                $(#[$($variant_attr:meta)*])*
                const $variant:ident = $value:expr;
            )*
        }
    ) => {
        $(#[$($attr)*])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name($inner);

        impl $name {
            $(
                $(#[$($variant_attr)*])*
                pub const $variant: Self = Self($value);
            )*

            #[doc = ::core::concat!("Creates a new [`", stringify!($name), "`] from the provided raw value.")]
            #[inline]
            pub fn from_raw(raw: $inner) -> Self {
                Self(raw)
            }

            #[doc = ::core::concat!("Creates a new [`", stringify!($name), "`] from the provided known value.")]
            ///
            /// If the provided value is not a known variant, this function will return [`None`].
            pub fn from_known(raw: $inner) -> Option<Self> {
                match raw {
                    $(
                        $value => Some(Self($value)),
                    )*
                    _ => None,
                }
            }

            #[doc = ::core::concat!("Returns the raw value of this [`", stringify!($name), "`].")]
            #[inline]
            pub fn as_raw(self) -> $inner {
                self.0
            }

            #[doc = ::core::concat!("Returns whether this [`", stringify!($name), "`] is a known enum value.")]
            #[allow(clippy::manual_range_patterns)]
            pub fn is_known(self) -> bool {
                ::core::matches!(self.0, $(
                    | $value
                )*)
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self.0 {
                    $(
                        $value => write!(f, stringify!($variant)),
                    )*
                    _ => f.debug_tuple(stringify!($name)).field(&self.0).finish(),
                }
            }
        }
    }
}
