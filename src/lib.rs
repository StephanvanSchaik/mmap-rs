#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "windows"))]
mod areas;
pub mod error;
mod mmap;
mod os_impl;

#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "windows"))]
pub use areas::*;
pub use error::Error;
pub use mmap::*;
