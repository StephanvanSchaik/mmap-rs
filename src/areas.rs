use bitflags::bitflags;
use crate::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Range;
use std::path::PathBuf;

#[cfg(target_os = "freebsd")]
use crate::os_impl::freebsd as platform;

#[cfg(target_os = "linux")]
use crate::os_impl::linux as platform;

#[cfg(target_os = "windows")]
use crate::os_impl::windows as platform;

bitflags! {
    /// The protection flags of the memory area.
    pub struct ProtectionFlags: u32 {
        /// The memory area is mapped with read permissions.
        const READ          = 1 << 0;
        /// The memory area is mapped with write permissions.
        const WRITE         = 1 << 1;
        /// The memory area is mapped with execute permissions.
        const EXECUTE       = 1 << 3;
        /// The memory area is mapped with copy-on-write.
        const COPY_ON_WRITE = 1 << 4;
    }
}

/// Describes a memory area of a process.
#[derive(Clone, Debug)]
pub struct MemoryArea {
    /// The address range of the memory area.
    pub range: Range<usize>,
    /// The protection flags with which the memory area has been mapped.
    pub protection: ProtectionFlags,
    /// The path to the file that backs this memory area, if backed by a file.
    pub path: Option<(PathBuf, u64)>,
}

/// The memory maps of the process.
pub struct MemoryMaps<B> {
    inner: platform::MemoryMaps<B>,
}

impl MemoryMaps<BufReader<File>> {
    /// Creates an iterator over the memory maps for the specified process. If no process ID is
    /// given, then it enumerates the memory areas of the current process.
    pub fn open(pid: Option<u32>) -> Result<Self, Error> {
        let inner = platform::MemoryMaps::open(pid)?;

        Ok(Self {
            inner,
        })
    }
}

impl<B: BufRead> Iterator for MemoryMaps<B> {
    type Item = Result<MemoryArea, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
