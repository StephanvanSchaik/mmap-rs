#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

pub mod error;
mod mmap;
mod os_impl;

#[cfg(unix)]
pub(crate) use os_impl::unix as platform;

#[cfg(windows)]
pub(crate) use os_impl::windows as platform;

pub use mmap::*;
