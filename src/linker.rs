//! This module provides ways to access the address of some symbols specified in the linker script.

use core::ptr::addr_of;

/// Evaluates to the address of the given symbol.
macro addr_of_symbol($symbol:ident) {{
    extern "C" {
        static $symbol: u8;
    }

    unsafe { addr_of!($symbol) }
}}

/// Returns the address of the kernel's base address in virtual memory.
#[inline]
pub fn kernel_image_begin() -> *const u8 {
    addr_of_symbol!(__ruel_image_begin)
}

/// Returns the address of the kernel's end address in virtual memory.
#[inline]
pub fn kernel_image_end() -> *const u8 {
    addr_of_symbol!(__ruel_image_end)
}
