# Changelog

All notable changes to mmap-rs will be documented in this file.

## 0.6.0

- Add `MemoryAreas::query()` and related functions to query a specific address or address range 
- Add `Mmap::split_off()` and related functions to split an existing memory mapping at a page boundary.
- Add `MmapOptions::reserve()` and related functions to reserve memory mappings instead of committing them.
- Change `MmapOptions::with_file()` to require a reference to the file instead of taking ownership instead.
- Document flags that may have no operation on platforms that do not support the flag, such as `MmapFlags::STACK` on Microsoft Windows.
- Use `mach_vm_region_recurse()` on MacOS to recurse into submappings.

## 0.5.0

- Implement `MmapOptions::page_sizes()` to return the supported page sizes for the platform.
- Separate the functions to get the page size and the allocation granularity.
- Updated windows crate from version 0.39 to 0.44.
- Changed `MmapOptions::new()` to return `Result<Self, Error>` rather than `Self` to support the use of `NonZeroUsize` in nix.
- Updated nix crate from version 0.24 to 0.26.
- Added support for `i686-linux-android`, `aarch64-linux-android`, `x86_64-linux-android` and `armv7-linux-androideabi`.
- Added `MapFlags::TRANSPARENT_HUGE_PAGES` to hint the kernel that it may merge pages within the mapping into huge pages if possible when set to `madvise` mode.
- `MmapOptions::with_flags()` appends the flags instead of overriding them.
- Implement `Send` and `Sync` for `Mmap`.

## 0.4.0

- Added support for `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `aarch64-apple-ios`, `x86_64-apple-ios`, `armv7a-unknown-linux-gnueabihf`, `aarch64-unknown-linux-gnu` and `i686-unknown-linux-gnu`.
- Updated various dependencies.
- Relicensed under MIT and Apache.
- Added the `MemoryAreas` iterator to iterate over the memory areas of a process.
- Changed `UnsafeMmapFlags::MAP_JIT` to `UnsafeMmapFlags::JIT` to fix compilation on Mac OS X.
