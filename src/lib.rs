#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

mod areas;
pub mod error;
mod mmap;
mod os_impl;

#[cfg(target_os = "linux")]
pub use areas::*;

pub use error::Error;
pub use mmap::*;
