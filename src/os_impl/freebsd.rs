use bitflags::bitflags;
use crate::areas::{MemoryArea, Protection, ShareMode};
use crate::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::Path;

bitflags! {
    pub struct KvmeProtection: libc::c_int {
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

bitflags! {
    pub struct KvmeFlags: libc::c_int {
        const COW        = 1 << 0;
        const NEEDS_COPY = 1 << 1;
        const NOCOREDUMP = 1 << 2;
        const SUPER      = 1 << 3;
        const GROWS_UP   = 1 << 4;
        const GROWS_DOWN = 1 << 5;
        const USER_WIRED = 1 << 6;
    }
}

pub struct MemoryAreas<B> {
    entries: Vec<libc::kinfo_vmentry>,
    index: usize,
    marker: PhantomData<B>,
}

impl MemoryAreas<BufReader<File>> {
    pub fn open(pid: Option<u32>) -> Result<Self, Error> {
        // Default to the current process if no PID was specified.
        let pid = match pid {
            Some(pid) => pid as _,
            _ => unsafe { libc::getpid() },
        };

        let mut count = 0;
        let entries_ptr = unsafe {
            libc::kinfo_getvmmap(pid, &mut count)
        };

        let entries = unsafe {
            core::slice::from_raw_parts(entries_ptr, count as usize)
        }.to_vec();

        unsafe {
            libc::free(entries_ptr as *mut core::ffi::c_void);
        }

        Ok(Self {
            entries,
            index: 0,
            marker: PhantomData,
        })
    }
}

impl<B: BufRead> Iterator for MemoryAreas<B> {
    type Item = Result<MemoryArea, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.entries.len() {
            return None;
        }

        let entry = &self.entries[self.index];
        self.index += 1;

        let flags = KvmeProtection::from_bits_truncate(entry.kve_protection);

        let mut protection = Protection::empty();

        if flags.contains(KvmeProtection::READ) {
            protection |= Protection::READ;
        }

        if flags.contains(KvmeProtection::WRITE) {
            protection |= Protection::WRITE;
        }

        if flags.contains(KvmeProtection::EXECUTE) {
            protection |= Protection::EXECUTE;
        }

        let flags = KvmeFlags::from_bits_truncate(entry.kve_flags);

        let share_mode = if flags.contains(KvmeFlags::COW) {
            ShareMode::CopyOnWrite
        } else {
            ShareMode::Private
        };

        let start = entry.kve_start as usize;
        let end = entry.kve_end as usize;
        let offset = entry.kve_offset;

        // Parse the path.
        let path: Vec<u8> = entry.kve_path.iter().flatten().map(|byte| *byte as u8).collect();

        let last = match path.iter().position(|&c| c == 0) {
            Some(end) => end,
            _ => path.len(),
        };

        let path = if last == 0 {
            None
        } else {
            let path = match std::str::from_utf8(&path) {
                Ok(path) => path,
                Err(e) => return Some(Err(Error::Utf8(e))),
            };

            Some((Path::new(path).to_path_buf(), offset))
        };

        Some(Ok(MemoryArea {
            range: start..end,
            protection,
            share_mode,
            path,
        }))
    }
}
