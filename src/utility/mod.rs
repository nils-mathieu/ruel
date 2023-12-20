//! This module provides some simple utility functions and structures used across the kernel.

mod display;
use core::mem::MaybeUninit;

pub use self::display::*;

mod guards;
pub use self::guards::*;

pub mod array_vec;
pub use self::array_vec::FixedVec;

mod bump_allocator;
pub use self::bump_allocator::*;

pub mod stable_array_vec;
pub use self::stable_array_vec::StableFixedVec;

/// Attempts to initialize the provided slice by repeatedly calling the provided function.
pub fn try_init_slice_with<T, E>(
    slice: &mut [MaybeUninit<T>],
    mut new: impl FnMut(usize) -> Result<T, E>,
) -> Result<&mut [T], E> {
    let mut vec = FixedVec::new(slice);
    while !vec.is_full() {
        vec.push(new(vec.len())?);
    }
    Ok(vec.into_inner_slice())
}

/// Initializes the provided slice by repeatedly calling the provided function.
pub fn init_slice_with<T>(
    slice: &mut [MaybeUninit<T>],
    mut new: impl FnMut(usize) -> T,
) -> &mut [T] {
    match try_init_slice_with::<T, core::convert::Infallible>(slice, |index| Ok(new(index))) {
        Ok(slice) => slice,
        Err(err) => match err {},
    }
}
