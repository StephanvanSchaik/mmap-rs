# mmap-rs

[![Crates.io](https://img.shields.io/crates/v/mmap-rs.svg)](https://crates.io/crates/mmap-rs)
[![Docs](https://docs.rs/mmap-rs/badge.svg)](https://docs.rs/mmap-rs)

A cross-platform and safe Rust API to create and manage memory mappings in the virtual address
space of the calling process. This crate can be used to create both file mappings and anonymous
mappings. In addition, this crate supports the use of features such as huge pages, locking
physical memory, etc. on platforms where those features are available.

## Features

- [x] Anonymous memory maps.
- [x] File-backed memory maps (`unsafe` - see documentation for details).
- [x] Copy-on-write vs. shared memory maps.
- [x] Inaccessible memory maps (using `PROT_NONE` and `PAGE_NOACCESS`).
- [x] Read-only memory maps.
- [x] Read-write memory maps.
- [x] Executable memory maps.
- [x] RWX mmemory maps for JIT purposes (`unsafe` - see documentation for details).
- [x] Portable instruction cache flushing.
- [x] Synchronous and asynchronous flushing.
- [x] Support for locking physical memory.
- [x] Huge page support.
- [x] Stack support (also known as `MAP_STACK` on Unix).
- [x] Support to exclude memory maps from core dumps (on Unix only).
