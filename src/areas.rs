use crate::error::Error;
use bitflags::bitflags;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Range;
use std::path::PathBuf;

#[cfg(target_os = "freebsd")]
use crate::os_impl::freebsd as platform;

#[cfg(any(target_os = "android", target_os = "linux"))]
use crate::os_impl::linux as platform;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use crate::os_impl::macos as platform;

#[cfg(target_os = "windows")]
use crate::os_impl::windows as platform;

bitflags! {
    /// The protection of the memory area.
    pub struct Protection: u32 {
        /// The memory area is mapped with read permissions.
        const READ          = 1 << 0;
        /// The memory area is mapped with write permissions.
        const WRITE         = 1 << 1;
        /// The memory area is mapped with execute permissions.
        const EXECUTE       = 1 << 3;
    }
}

/// The share mode of the memory area.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShareMode {
    /// The memory area is mapped as private.
    Private,
    /// The memory area is mapped as copy-on-write.
    CopyOnWrite,
    /// The memory area is mapped as shared.
    Shared,
}

/// Describes a memory area of a process.
#[derive(Clone, Debug)]
pub struct MemoryArea {
    /// The address range of the memory area.
    pub(crate) range: Range<usize>,
    /// The protection with which the memory area has been mapped.
    pub(crate) protection: Protection,
    /// The share mode of the memory area.
    pub(crate) share_mode: ShareMode,
    /// The path to the file that backs this memory area, if backed by a file.
    pub(crate) path: Option<(PathBuf, u64)>,
}

impl MemoryArea {
    /// The address range of the memory area.
    #[inline]
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// The start address of the area.
    #[inline]
    pub fn start(&self) -> usize {
        self.range.start
    }

    /// The end address of the area.
    #[inline]
    pub fn end(&self) -> usize {
        self.range.end
    }

    /// The protection with which the memory area has been mapped.
    #[inline]
    pub fn protection(&self) -> Protection {
        self.protection
    }

    /// The share mode of the memory area.
    #[inline]
    pub fn share_mode(&self) -> ShareMode {
        self.share_mode
    }

    /// The path to the file that backs this memory area, if backed by a file.
    #[inline]
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref().map(|(path, _)| path)
    }

    /// The file offset, if backed by a file.
    #[inline]
    pub fn file_offset(&self) -> Option<u64> {
        self.path.as_ref().map(|(_, offset)| *offset)
    }
}

/// The memory areas of the process.
pub struct MemoryAreas<B> {
    inner: platform::MemoryAreas<B>,
}

impl<B> fmt::Debug for MemoryAreas<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryAreas").finish_non_exhaustive()
    }
}

impl MemoryAreas<BufReader<File>> {
    /// Creates an iterator over the memory maps for the specified process. If no process ID is
    /// given, then it enumerates the memory areas of the current process.
    pub fn open(pid: Option<u32>) -> Result<Self, Error> {
        let inner = platform::MemoryAreas::open(pid)?;

        Ok(Self { inner })
    }
}

impl<B: BufRead> Iterator for MemoryAreas<B> {
    type Item = Result<MemoryArea, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
