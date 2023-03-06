use crate::areas::{MemoryArea, Protection, ShareMode};
use crate::error::Error;
use crate::mmap::{MmapFlags, PageSize, PageSizes, UnsafeMmapFlags};
use bitflags::bitflags;
use std::fs::File;
use std::ops::Range;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, MAX_PATH};
#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
use windows::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows::Win32::System::Memory::*;
use windows::Win32::System::ProcessStatus::K32GetMappedFileNameW;
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_ALL_ACCESS};

bitflags! {
    struct Flags: u32 {
        const COPY_ON_WRITE = 1 << 0;
        const JIT           = 1 << 1;
    }
}

pub struct Mmap {
    file: Option<File>,
    ptr: *mut u8,
    size: usize,
    flags: Flags,
}

unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl Mmap {
    #[inline]
    pub fn file(&self) -> Option<&File> {
        self.file.as_ref()
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn lock(&mut self) -> Result<(), Error> {
        let status =
            unsafe { VirtualLock(self.ptr as *const std::ffi::c_void, self.size) }.as_bool();

        if !status {
            return Err(std::io::Error::last_os_error())?;
        }

        Ok(())
    }

    pub fn unlock(&mut self) -> Result<(), Error> {
        let status =
            unsafe { VirtualUnlock(self.ptr as *const std::ffi::c_void, self.size) }.as_bool();

        if !status {
            return Err(std::io::Error::last_os_error())?;
        }

        Ok(())
    }

    pub fn flush(&self, range: Range<usize>) -> Result<(), Error> {
        self.flush_async(range)?;

        if let Some(ref file) = self.file {
            file.sync_data()?;
        }

        Ok(())
    }

    pub fn flush_async(&self, range: Range<usize>) -> Result<(), Error> {
        if range.end <= range.start {
            return Ok(());
        }

        let status = unsafe {
            FlushViewOfFile(
                self.ptr.offset(range.start as isize) as *const std::ffi::c_void,
                range.end - range.start,
            )
        }
        .as_bool();

        if !status {
            return Err(std::io::Error::last_os_error())?;
        }

        Ok(())
    }

    pub fn do_make(&self, protect: PAGE_PROTECTION_FLAGS) -> Result<(), Error> {
        let mut old_protect = PAGE_PROTECTION_FLAGS::default();

        let status = unsafe {
            VirtualProtect(
                self.ptr as *mut std::ffi::c_void,
                self.size,
                protect,
                &mut old_protect,
            )
            .as_bool()
        };

        if !status {
            return Err(std::io::Error::last_os_error())?;
        }

        Ok(())
    }

    pub fn flush_icache(&self) -> Result<(), Error> {
        // While the x86 and x86-64 architectures guarantee cache coherency between the L1
        // instruction and the L1 data cache, other architectures such as arm and aarch64 do not.
        // If the user modified the pages, then executing the code after marking the pages as
        // executable may result in undefined behavior. Since we cannot efficiently track writes,
        // we have to flush the instruction cache unconditionally.
        #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
        unsafe {
            FlushInstructionCache(
                GetCurrentProcess(),
                Some(self.ptr as *const std::ffi::c_void),
                self.size,
            )
        };

        Ok(())
    }

    pub fn make_none(&self) -> Result<(), Error> {
        self.do_make(PAGE_NOACCESS)
    }

    pub fn make_read_only(&self) -> Result<(), Error> {
        self.do_make(PAGE_READWRITE)
    }

    pub fn make_exec(&self) -> Result<(), Error> {
        self.do_make(PAGE_EXECUTE_READ)
    }

    pub fn make_mut(&self) -> Result<(), Error> {
        let protect = if self.file.is_some() && self.flags.contains(Flags::COPY_ON_WRITE) {
            PAGE_WRITECOPY
        } else {
            PAGE_READWRITE
        };

        self.do_make(protect)
    }

    pub fn make_exec_mut(&self) -> Result<(), Error> {
        if !self.flags.contains(Flags::JIT) {
            return Err(Error::UnsafeFlagNeeded(UnsafeMmapFlags::JIT));
        }

        let protect = if self.flags.contains(Flags::COPY_ON_WRITE) {
            PAGE_EXECUTE_WRITECOPY
        } else {
            PAGE_EXECUTE_READWRITE
        };

        self.do_make(protect)
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        if self.file.is_some() {
            let _ = unsafe { UnmapViewOfFile(self.ptr as *mut _) };
        } else {
            let _ = unsafe {
                VirtualFree(
                    self.ptr as *mut _,
                    self.size,
                    // FIXME: for some reason BitOr is not implemented for VIRTUAL_FREE_TYPE.
                    VIRTUAL_FREE_TYPE(MEM_DECOMMIT.0 | MEM_RELEASE.0),
                )
            };
        }
    }
}

pub struct MmapOptions {
    address: Option<usize>,
    file: Option<(File, u64)>,
    size: usize,
    flags: MmapFlags,
    unsafe_flags: UnsafeMmapFlags,
    page_size: Option<PageSize>,
}

impl MmapOptions {
    pub fn new(size: usize) -> Result<Self, Error> {
        Ok(Self {
            address: None,
            file: None,
            size,
            flags: MmapFlags::empty(),
            unsafe_flags: UnsafeMmapFlags::empty(),
            page_size: None,
        })
    }

    pub fn page_size() -> usize {
        let mut system_info = SYSTEM_INFO::default();

        unsafe { GetSystemInfo(&mut system_info) };

        system_info.dwPageSize as usize
    }

    pub fn page_sizes() -> Result<PageSizes, Error> {
        let mut sizes = 1 << Self::page_size().ilog2();

        let size = unsafe { GetLargePageMinimum() };

        if size != 0 {
            sizes |= 1 << size.ilog2();
        }

        Ok(PageSizes::from_bits_truncate(sizes))
    }

    pub fn allocation_granularity() -> usize {
        let mut system_info = SYSTEM_INFO::default();

        unsafe { GetSystemInfo(&mut system_info) };

        system_info.dwAllocationGranularity as usize
    }

    pub fn with_address(mut self, address: usize) -> Self {
        self.address = Some(address);
        self
    }

    pub fn with_file(mut self, file: File, offset: u64) -> Self {
        self.file = Some((file, offset));
        self
    }

    pub fn with_flags(mut self, flags: MmapFlags) -> Self {
        self.flags |= flags;
        self
    }

    pub unsafe fn with_unsafe_flags(mut self, flags: UnsafeMmapFlags) -> Self {
        self.unsafe_flags |= flags;
        self
    }

    pub fn with_page_size(mut self, page_size: PageSize) -> Self {
        self.page_size = Some(page_size);
        self
    }

    /// This is a helper function that simply calls [`CreateFileMappingW`] and then [`CloseHandle`]
    /// to check if a file mapping can be created with the given protection. This is mostly needed
    /// to figure out whether a file mapping can be created with read, write and execute access.
    /// Returns true on success and false otherwise.
    fn check_protection(&self, protection: PAGE_PROTECTION_FLAGS) -> bool {
        // Grab a reference to the file, if there is one. Otherwise return false immediately.
        let file = match self.file.as_ref() {
            Some((file, _)) => file,
            _ => return false,
        };

        // Try creating a file mapping with the given protection.
        let file_mapping = unsafe {
            CreateFileMappingW(
                HANDLE(file.as_raw_handle() as isize),
                None,
                protection,
                0,
                0,
                PCWSTR::null(),
            )
        };

        let file_mapping = match file_mapping {
            Ok(file_mapping) => file_mapping,
            _ => return false,
        };

        // Return false if we could not create the mapping.
        if file_mapping.is_invalid() {
            return false;
        }

        // We could create the file mapping, now close the handle and return true.
        unsafe { CloseHandle(file_mapping) };

        true
    }

    /// This is a helper function that goes through the process of setting up the desired memory
    /// mapping given the protection flag.
    fn do_map(mut self, protection: PAGE_PROTECTION_FLAGS) -> Result<Mmap, Error> {
        // We have to check whether we can create the file mapping with write and execute
        // permissions. As Microsoft Windows won't let us set any access flags other than those
        // that have been set initially, we have to figure out the full set of access flags that
        // we can set, and then narrow down the access rights to what the user requested.
        let write = self.check_protection(PAGE_READWRITE);
        let execute = self.check_protection(PAGE_EXECUTE_READ);

        let mut map_access = FILE_MAP_READ;
        let mut map_protection = match (write, execute) {
            (true, true) => {
                map_access |= FILE_MAP_WRITE | FILE_MAP_EXECUTE;
                PAGE_EXECUTE_READWRITE
            }
            (true, false) => {
                map_access |= FILE_MAP_WRITE;
                PAGE_READWRITE
            }
            (false, true) => {
                map_access |= FILE_MAP_EXECUTE;
                PAGE_EXECUTE_READ
            }
            (false, false) => PAGE_READONLY,
        };

        let size = self.size;
        let ptr = if let Some((file, offset)) = &self.file {
            if self.flags.contains(MmapFlags::HUGE_PAGES) {
                map_access |= FILE_MAP_LARGE_PAGES;
                map_protection |= SEC_LARGE_PAGES;
            }

            let file_mapping = unsafe {
                CreateFileMappingW(
                    HANDLE(file.as_raw_handle() as isize),
                    None,
                    map_protection,
                    (match size.overflowing_shr(32) {
                        (_, true) => 0,
                        (size, false) => size,
                    } & 0xffff_ffff) as u32,
                    (size & 0xffff_ffff) as u32,
                    PCWSTR::null(),
                )
            }?;

            let ptr = unsafe {
                MapViewOfFileEx(
                    file_mapping,
                    map_access,
                    ((offset >> 32) & 0xffff_ffff) as u32,
                    (offset & 0xffff_ffff) as u32,
                    size,
                    None,
                )
            };

            unsafe { CloseHandle(file_mapping) };

            let mut old_protect = PAGE_PROTECTION_FLAGS::default();

            let status =
                unsafe { VirtualProtect(ptr, size, protection, &mut old_protect) }.as_bool();

            if !status {
                return Err(std::io::Error::last_os_error())?;
            }

            ptr
        } else {
            let mut flags = MEM_COMMIT | MEM_RESERVE;

            if self.flags.contains(MmapFlags::HUGE_PAGES) {
                flags |= MEM_LARGE_PAGES;
            }

            unsafe {
                VirtualAlloc(
                    self.address
                        .map(|address| address as *const std::ffi::c_void),
                    size,
                    flags,
                    protection,
                )
            }
        };

        if ptr.is_null() {
            return Err(std::io::Error::last_os_error())?;
        }

        let size = self.size;
        let file = self.file.take().map(|(file, _)| file);
        let mut flags = Flags::empty();

        if self.flags.contains(MmapFlags::COPY_ON_WRITE) {
            flags |= Flags::COPY_ON_WRITE;
        }

        if self.unsafe_flags.contains(UnsafeMmapFlags::JIT) {
            flags |= Flags::JIT;
        }

        Ok(Mmap {
            file,
            ptr: ptr as *mut u8,
            size,
            flags,
        })
    }

    pub fn map_none(self) -> Result<Mmap, Error> {
        self.do_map(PAGE_NOACCESS)
    }

    pub fn map(self) -> Result<Mmap, Error> {
        self.do_map(PAGE_READONLY)
    }

    pub fn map_exec(self) -> Result<Mmap, Error> {
        self.do_map(PAGE_EXECUTE_READ)
    }

    pub fn map_mut(self) -> Result<Mmap, Error> {
        let protect = if self.file.is_some() && self.flags.contains(MmapFlags::COPY_ON_WRITE) {
            PAGE_WRITECOPY
        } else {
            PAGE_READWRITE
        };

        self.do_map(protect)
    }

    pub fn map_exec_mut(self) -> Result<Mmap, Error> {
        if !self.unsafe_flags.contains(UnsafeMmapFlags::JIT) {
            return Err(Error::UnsafeFlagNeeded(UnsafeMmapFlags::JIT));
        }

        let protect = if self.flags.contains(MmapFlags::COPY_ON_WRITE) {
            PAGE_EXECUTE_WRITECOPY
        } else {
            PAGE_EXECUTE_READWRITE
        };

        self.do_map(protect)
    }
}

use std::io::{BufRead, BufReader};
use std::marker::PhantomData;

pub struct MemoryAreas<B> {
    handle: HANDLE,
    address: usize,
    marker: PhantomData<B>,
}

impl MemoryAreas<BufReader<File>> {
    pub fn open(pid: Option<u32>) -> Result<Self, Error> {
        let handle = match pid {
            Some(id) => unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, id) }?,
            _ => unsafe { GetCurrentProcess() },
        };

        Ok(Self {
            handle,
            address: 0,
            marker: PhantomData,
        })
    }
}

impl<B: BufRead> Iterator for MemoryAreas<B> {
    type Item = Result<MemoryArea, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut info = MEMORY_BASIC_INFORMATION::default();

        loop {
            let address = self.address;

            let size = unsafe {
                VirtualQueryEx(
                    self.handle,
                    Some(address as _),
                    &mut info,
                    std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };

            if size < std::mem::size_of::<MEMORY_BASIC_INFORMATION>() {
                return None;
            }

            let size = info.RegionSize as usize;
            let start = info.BaseAddress as usize;
            let end = start + size;
            let range = start..end;

            self.address += size;

            if info.State & MEM_COMMIT == VIRTUAL_ALLOCATION_TYPE(0) {
                continue;
            }

            let copy_on_write =
                info.Protect == PAGE_EXECUTE_WRITECOPY || info.Protect == PAGE_WRITECOPY;

            let share_mode = if info.Type & MEM_PRIVATE == MEM_PRIVATE {
                ShareMode::Private
            } else if copy_on_write {
                ShareMode::CopyOnWrite
            } else {
                ShareMode::Shared
            };

            let protection = match info.Protect {
                PAGE_EXECUTE => Protection::EXECUTE,
                PAGE_EXECUTE_READ => Protection::READ | Protection::EXECUTE,
                PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY => {
                    Protection::READ | Protection::WRITE | Protection::EXECUTE
                }
                PAGE_READONLY => Protection::READ,
                PAGE_READWRITE | PAGE_WRITECOPY => Protection::READ | Protection::WRITE,
                _ => Protection::empty(),
            };

            let mut name = vec![0u16; MAX_PATH as usize];

            let name_size = unsafe {
                K32GetMappedFileNameW(self.handle, address as *const std::ffi::c_void, &mut name)
            };

            let path = if name_size != 0 {
                let path = widestring::U16CStr::from_slice_truncate(&name).unwrap();
                let path = path.to_string_lossy();

                let offset = (info.BaseAddress as u64) - (info.AllocationBase as u64);

                Some((PathBuf::from(path), offset))
            } else {
                None
            };

            return Some(Ok(MemoryArea {
                range,
                protection,
                share_mode,
                path,
            }));
        }
    }
}
