/// A wrapper around a `u64` that displays it as a human-readable byte count.
pub struct HumanByteCount(pub u64);

#[inline]
fn write_dec(f: &mut core::fmt::Formatter, n: u64, dim: &str) -> core::fmt::Result {
    let dec = ((n % 1024) * 100) / 1024;
    if dec == 0 {
        write!(f, "{} {}", n / 1024, dim)
    } else {
        write!(f, "{}.{} {}", n / 1024, dec, dim)
    }
}

impl core::fmt::Display for HumanByteCount {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let mut bytes = self.0;

        if bytes < 1024 {
            return write!(f, "{} B", bytes);
        }

        if bytes < 1024 * 1024 {
            return write_dec(f, bytes, "KiB");
        }

        bytes /= 1024;

        if bytes < 1024 * 1024 {
            return write_dec(f, bytes, "MiB");
        }

        bytes /= 1024;

        if bytes < 1024 * 1024 {
            return write_dec(f, bytes, "GiB");
        }

        bytes /= 1024;

        // That's a lot of memory.
        write_dec(f, bytes, "TiB")
    }
}
