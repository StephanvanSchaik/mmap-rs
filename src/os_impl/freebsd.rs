use bitflags::bitflags;
use crate::areas::{MemoryArea, ProtectionFlags};
use crate::error::Error;
use nix::unistd::getpid;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::Path;
use sysctl::{CtlValue, Sysctl};

bitflags! {
    pub struct KvmeProtection: u32 {
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

bitflags! {
    pub struct KvmeFlags: u32 {
        const COW        = 1 << 0;
        const NEEDS_COPY = 1 << 1;
        const NOCOREDUMP = 1 << 2;
        const SUPER      = 1 << 3;
        const GROWS_UP   = 1 << 4;
        const GROWS_DOWN = 1 << 5;
        const USER_WIRED = 1 << 6;
    }
}

pub struct MemoryMaps<B> {
    bytes: Vec<u8>,
    marker: PhantomData<B>,
}

impl MemoryMaps<BufReader<File>> {
    pub fn open(pid: Option<u32>) -> Result<Self, Error> {
        let mut ctl = sysctl::Ctl::new("kern.proc.vmmap").unwrap();

        // Default to the current process if no PID was specified.
        let pid = match pid {
            Some(pid) => pid,
            _ => getpid().as_raw() as u32,
        };

        // Push the PID as part of the oid to query the VM map of the process.
        ctl.oid.push(pid as i32);

        let bytes = match ctl.value() {
            Ok(CtlValue::Node(bytes)) => bytes,
            Ok(_) => panic!("unexpected"),
            Err(e) => return Err(Error::Sysctl(e)),
        };

        Ok(Self {
            bytes,
            marker: PhantomData,
        })
    }
}

impl<B: BufRead> Iterator for MemoryMaps<B> {
    type Item = Result<MemoryArea, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // Parsing the entry is based on `struct kinfo_vmentry` as defined by
        // /usr/include/sys/user.h.

        // Parse the entry size.
        if self.bytes.len() < 4 {
            return None;
        }

        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&self.bytes[0..4]);
        let entry_size = u32::from_ne_bytes(bytes) as usize;

        // Parse the protection flags and entry flags.
        bytes.copy_from_slice(&self.bytes[56..60]);
        let flags = KvmeProtection::from_bits(u32::from_ne_bytes(bytes)).unwrap();

        let mut protection = ProtectionFlags::empty();

        if flags.contains(KvmeProtection::READ) {
            protection |= ProtectionFlags::READ;
        }

        if flags.contains(KvmeProtection::WRITE) {
            protection |= ProtectionFlags::WRITE;
        }

        if flags.contains(KvmeProtection::EXECUTE) {
            protection |= ProtectionFlags::EXECUTE;
        }

        bytes.copy_from_slice(&self.bytes[60..64]);
        let flags = KvmeFlags::from_bits(u32::from_ne_bytes(bytes)).unwrap();

        if flags.contains(KvmeFlags::COW) {
            protection |= ProtectionFlags::COPY_ON_WRITE;
        }

        // Parse the start address.
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.bytes[8..16]);
        let start = u64::from_ne_bytes(bytes) as usize;

        // Parse the end address.
        bytes.copy_from_slice(&self.bytes[16..24]);
        let end = u64::from_ne_bytes(bytes) as usize;

        // Parse the offset.
        bytes.copy_from_slice(&self.bytes[24..32]);
        let offset = u64::from_ne_bytes(bytes);

        // Parse the path.
        let path = &self.bytes[136..];

        let last = match path.iter().position(|&c| c == 0) {
            Some(end) => end,
            _ => path.len(),
        };

        let path = if last == 0 {
            None
        } else {
            Some((Path::new(std::str::from_utf8(&self.bytes[136..136 + last]).unwrap()).to_path_buf(), offset))
        };

        // Drain the bytes for this entry.
        self.bytes.drain(..entry_size);

        Some(Ok(MemoryArea {
            range: start..end,
            protection,
            path,
        }))
    }
}
