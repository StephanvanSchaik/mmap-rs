# Changelog

All notable changes to mmap-rs will be documented in this file.

## 0.4.0

- Added support for `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `aarch64-apple-ios`, `x86_64-apple-ios`, `armv7a-unknown-linux-gnueabihf`, `aarch64-unknown-linux-gnu` and `i686-unknown-linux-gnu`.
- Updated various dependencies.
- Relicensed under MIT and Apache.
- Added the `MemoryAreas` iterator to iterate over the memory areas of a process.
- Changed `UnsafeMmapFlags::MAP_JIT` to `UnsafeMmapFlags::JIT` to fix compilation on Mac OS X.
