//! The present documentation attempt to explain the inner workings of the Ruel Operating System.
//!
//! More background on the project can be found in the [README] file of the project.
//!
//! [README]: https://github.com/nils-mathieu/ruel/blob/master/README.md

//
// Crate-level attributes
//
#![no_std]
#![no_main]
//
// Lints
//
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
//
// Features
//
#![feature(used_with_arg)]
#![feature(decl_macro)]
#![feature(panic_info_message)]

mod boot;
mod hcf;
mod log;
mod sync;
mod utility;
