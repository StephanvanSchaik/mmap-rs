use bitflags::bitflags;
use crate::error::Error;
use std::fs::File;
use std::ops::{Deref, DerefMut, Range};

#[cfg(unix)]
use crate::os_impl::unix as platform;

#[cfg(windows)]
use crate::os_impl::windows as platform;

bitflags! {
    /// The available flags to configure the allocated mapping.
    pub struct MmapFlags: u32 {
        /// May initially map the pages as shared between multiple mappings, but creates a private
        /// copy when writing to the pages such that any modifications are not visible to any other
        /// processes.
        const COPY_ON_WRITE          = 1 << 0;

        /// Ensure the allocated pages are populated, such that they do not cause page faults.
        const POPULATE               = 1 << 1;

        /// Do not reserve swap space for this allocation.
        const NO_RESERVE             = 1 << 2;

        /// Use huge pages for this allocation.
        const HUGE_PAGES             = 1 << 3;

        /// The region grows downward like a stack.
        const STACK                  = 1 << 4;

        /// The pages will not be included in a core dump.
        const NO_CORE_DUMP           = 1 << 5;

        /// Lock the physical memory to prevent page faults from happening when accessing the
        /// pages.
        const LOCKED                 = 1 << 6;

        /// Suggest to use transparent huge pages for this allocation by calling madvise().
        const TRANSPARENT_HUGE_PAGES = 1 << 7;
    }

    /// The available flags to configure the allocated mapping, but that are considered unsafe to
    /// use.
    pub struct UnsafeMmapFlags: u32 {
        /// Maps the memory mapping at the address specified, replacing any pages that have been
        /// mapped at that address range.
        ///
        /// This is not supported on Microsoft Windows.
        const MAP_FIXED = 1 << 0;

        /// Allows mapping the page as RWX. While this may seem useful for self-modifying code and
        /// JIT engines, it is instead recommended to convert between mutable and executable
        /// mappings using [`Mmap::make_mut()`] and [`MmapMut::make_exec()`] instead.
        ///
        /// As it may be tempting to use this flag, this flag has been (indirectly) marked as
        /// **unsafe**. Make sure to read the text below to understand the complications of this
        /// flag before using it.
        ///
        /// RWX pages are an interesting targets to attackers, e.g. for buffer overflow attacks, as
        /// RWX mappings can potentially simplify such attacks. Without RWX mappings, attackers
        /// instead have to resort to return-oriented programming (ROP) gadgets. To prevent buffer
        /// overflow attacks, contemporary CPUs allow pages to be marked as non-executable which is
        /// then used by the operating system to ensure that pages are either marked as writeable
        /// or as executable, but not both. This is also known as W^X.
        ///
        /// While the x86 and x86-64 architectures guarantee cache coherency between the L1
        /// instruction and the L1 data cache, other architectures such as Arm and AArch64 do not.
        /// If the user modified the pages, then executing the code may result in undefined
        /// behavior. To ensure correct behavior a user has to flush the instruction cache after
        /// modifying and before executing the page.
        const JIT       = 1 << 1;
    }
}

/// The preferred size of the pages uses, where the size is in log2 notation.
///
/// Note that not all the offered page sizes may be available on the current platform.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PageSize(pub usize);

impl PageSize {
    /// Map the mapping using 4 KiB pages.
    pub const _4K:   Self = Self(12);
    /// Map the mapping using 64 KiB pages.
    pub const _64K:  Self = Self(16);
    /// Map the mapping using 512 KiB pages.
    pub const _512K: Self = Self(19);
    /// Map the mapping using 1 MiB pages.
    pub const _1M:   Self = Self(20);
    /// Map the mapping using 2 MiB pages.
    pub const _2M:   Self = Self(21);
    /// Map the mapping using 4 MiB pages.
    pub const _4M:   Self = Self(22);
    /// Map the mapping using 8 MiB pages.
    pub const _8M:   Self = Self(23);
    /// Map the mapping using 16 MiB pages.
    pub const _16M:  Self = Self(24);
    /// Map the mapping using 32 MiB pages.
    pub const _32M:  Self = Self(25);
    /// Map the mapping using 256 MiB pages.
    pub const _256M: Self = Self(28);
    /// Map the mapping using 512 MiB pages.
    pub const _512M: Self = Self(29);
    /// Map the mapping using 1 GiB pages.
    pub const _1G:   Self = Self(30);
    /// Map the mapping using 2 GiB pages.
    pub const _2G:   Self = Self(31);
    /// Map the mapping using 16 GiB pages.
    pub const _16G:  Self = Self(34);
}

macro_rules! mmap_impl {
    ($t:ident) => {
        impl $t {
            /// Yields the file backing this mapping, if this mapping is backed by a file.
            #[inline]
            pub fn file(&self) -> Option<&File> {
                self.inner.file()
            }

            /// Yields a raw immutable pointer of this mapping.
            #[inline]
            pub fn as_ptr(&self) -> *const u8 {
                self.inner.as_ptr()
            }

            /// Yields the size of this mapping.
            #[inline]
            pub fn size(&self) -> usize {
                self.inner.size()
            }

            /// Locks the physical pages in memory such that accessing the mapping causes no page faults.
            pub fn lock(&mut self) -> Result<(), Error> {
                self.inner.lock()
            }

            /// Unlocks the physical pages in memory, allowing the operating system to swap out the pages
            /// backing this memory mapping.
            pub fn unlock(&mut self) -> Result<(), Error> {
                self.inner.unlock()
            }

            /// Flushes the memory mapping synchronously, i.e. this function waits for the flush to
            /// complete.
            pub fn flush(&self, range: Range<usize>) -> Result<(), Error> {
                self.inner.flush(range)
            }

            /// Flushes the memory mapping asynchronously.
            pub fn flush_async(&self, range: Range<usize>) -> Result<(), Error> {
                self.inner.flush_async(range)
            }

            /// This function can be used to flush the instruction cache on architectures where
            /// this is required.
            ///
            /// While the x86 and x86-64 architectures guarantee cache coherency between the L1 instruction
            /// and the L1 data cache, other architectures such as Arm and AArch64 do not. If the user
            /// modified the pages, then executing the code may result in undefined behavior. To ensure
            /// correct behavior a user has to flush the instruction cache after modifying and before
            /// executing the page.
            pub fn flush_icache(&self) -> Result<(), Error> {
                self.inner.flush_icache()
            }

            /// Remaps this memory mapping as inaccessible.
            ///
            /// In case of failure, this returns the ownership of `self`.
            pub fn make_none(self) -> Result<MmapNone, (Self, Error)> {
                if let Err(e) = self.inner.make_none() {
                    return Err((self, e));
                }

                Ok(MmapNone {
                    inner: self.inner,
                })
            }

            /// Remaps this memory mapping as immutable.
            ///
            /// In case of failure, this returns the ownership of `self`.
            pub fn make_read_only(self) -> Result<Mmap, (Self, Error)> {
                if let Err(e) = self.inner.make_read_only() {
                    return Err((self, e));
                }

                Ok(Mmap {
                    inner: self.inner,
                })
            }

            /// Remaps this memory mapping as executable.
            ///
            /// In case of failure, this returns the ownership of `self`.
            pub fn make_exec(self) -> Result<Mmap, (Self, Error)> {
                if let Err(e) = self.inner.make_exec() {
                    return Err((self, e));
                }

                if let Err(e) = self.inner.flush_icache() {
                    return Err((self, e));
                }

                Ok(Mmap {
                    inner: self.inner,
                })
            }

            /// Remaps this memory mapping as executable, but does not flush the instruction cache.
            /// Note that this is **unsafe**.
            ///
            /// While the x86 and x86-64 architectures guarantee cache coherency between the L1 instruction
            /// and the L1 data cache, other architectures such as Arm and AArch64 do not. If the user
            /// modified the pages, then executing the code may result in undefined behavior. To ensure
            /// correct behavior a user has to flush the instruction cache after modifying and before
            /// executing the page.
            ///
            /// In case of failure, this returns the ownership of `self`.
            pub unsafe fn make_exec_no_flush(self) -> Result<Mmap, (Self, Error)> {
                if let Err(e) = self.inner.make_exec() {
                    return Err((self, e));
                }

                Ok(Mmap {
                    inner: self.inner,
                })
            }


            /// Remaps this mapping to be mutable.
            ///
            /// In case of failure, this returns the ownership of `self`.
            pub fn make_mut(self) -> Result<MmapMut, (Self, Error)> {
                if let Err(e) = self.inner.make_mut() {
                    return Err((self, e));
                }

                Ok(MmapMut {
                    inner: self.inner,
                })
            }

            /// Remaps this mapping to be executable and mutable.
            ///
            /// While this may seem useful for self-modifying
            /// code and JIT engines, it is instead recommended to convert between mutable and executable
            /// mappings using [`Mmap::make_mut()`] and [`MmapMut::make_exec()`] instead.
            ///
            /// As it may be tempting to use this function, this function has been marked as **unsafe**.
            /// Make sure to read the text below to understand the complications of this function before
            /// using it. The [`UnsafeMmapFlags::JIT`] flag must be set for this function to succeed.
            ///
            /// RWX pages are an interesting targets to attackers, e.g. for buffer overflow attacks, as RWX
            /// mappings can potentially simplify such attacks. Without RWX mappings, attackers instead
            /// have to resort to return-oriented programming (ROP) gadgets. To prevent buffer overflow
            /// attacks, contemporary CPUs allow pages to be marked as non-executable which is then used by
            /// the operating system to ensure that pages are either marked as writeable or as executable,
            /// but not both. This is also known as W^X.
            ///
            /// While the x86 and x86-64 architectures guarantee cache coherency between the L1 instruction
            /// and the L1 data cache, other architectures such as Arm and AArch64 do not. If the user
            /// modified the pages, then executing the code may result in undefined behavior. To ensure
            /// correct behavior a user has to flush the instruction cache after modifying and before
            /// executing the page.
            ///
            /// In case of failure, this returns the ownership of `self`.
            pub unsafe fn make_exec_mut(self) -> Result<MmapMut, (Self, Error)> {
                if let Err(e) = self.inner.make_exec_mut() {
                    return Err((self, e));
                }

                Ok(MmapMut {
                    inner: self.inner,
                })
            }
        }
    }
}

/// Represents an inaccessible memory mapping.
pub struct MmapNone {
    inner: platform::Mmap,
}

mmap_impl!(MmapNone);

/// Represents an immutable memory mapping.
pub struct Mmap {
    inner: platform::Mmap,
}

mmap_impl!(Mmap);

impl Mmap {
    /// Extracts a slice containing the entire mapping.
    ///
    /// This is equivalent to `&mapping[..]`.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self[..]
    }
}

impl Deref for Mmap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(self.as_ptr(), self.size())
        }
    }
}

impl AsRef<[u8]> for Mmap {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.as_ptr(), self.size())
        }
    }
}

/// Represents a mutable memory mapping.
pub struct MmapMut {
    inner: platform::Mmap,
}

mmap_impl!(MmapMut);

impl MmapMut {
    /// Extracts a slice containing the entire mapping.
    ///
    /// This is equivalent to `&mapping[..]`.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self[..]
    }

    /// Extracts a mutable slice containing the entire mapping.
    ///
    /// This is equivalent to `&mut mapping[..]`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self[..]
    }

    /// Yields a raw mutable pointer to this mapping.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.inner.as_mut_ptr()
    }
}

impl Deref for MmapMut {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(self.as_ptr(), self.size())
        }
    }
}

impl DerefMut for MmapMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.size())
        }
    }
}

impl AsRef<[u8]> for MmapMut {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.as_ptr(), self.size())
        }
    }
}

impl AsMut<[u8]> for MmapMut {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.size())
        }
    }
}

/// Represents the options for the memory mapping.
pub struct MmapOptions {
    inner: platform::MmapOptions,
}

impl MmapOptions {
    /// Constructs the `MmapOptions` builder. The size specified is the size of the mapping to be
    /// allocated in bytes.
    pub fn new(size: usize) -> Result<Self, Error> {
        Ok(Self {
            inner: platform::MmapOptions::new(size)?,
        })
    }

    /// Returns the smallest possible page size for the current platform. The allocation size must
    /// be aligned to the page size for the allocation to succeed.
    pub fn page_size() -> usize {
        platform::MmapOptions::page_size()
    }

    /// Returns the allocation granularity for the current platform. On some platforms the
    /// allocation granularity may be a multiple of the page size. The start address of the
    /// allocation must be aligned to `max(allocation_granularity, page_size)`.
    pub fn allocation_granularity() -> usize {
        platform::MmapOptions::allocation_granularity()
    }

    /// The desired address at which the memory should be mapped.
    pub fn with_address(self, address: usize) -> Self {
        Self {
            inner: self.inner.with_address(address),
        }
    }

    /// Whether the memory mapping should be backed by a [`File`] or not. If the memory mapping
    /// should be mapped by a [`File`], then the user can also specify the offset within the file
    /// at which the mapping should start.
    ///
    /// On Microsoft Windows, it may not be possible to extend the protection beyond the access
    /// mask that has been used to open the file. For instance, if a file has been opened with read
    /// access, then [`Mmap::make_mut()`] will not work. Furthermore, [`std::fs::OpenOptions`] does
    /// not in itself provide a standardized way to open the file with executable access. However,
    /// if the file is not opened with executable access, then it may not be possible to use
    /// [`Mmap::make_exec()`]. Fortunately, Rust provides [`OpenOptionsExt`] that allows you to
    /// open the file with executable access rights. See [`access_mode`] for more information.
    ///
    /// This function is marked as **unsafe** as the user should be aware that even in the case
    /// that a file is mapped as immutable in the address space of the current process, it does not
    /// guarantee that there does not exist any other mutable mapping to the file.
    ///
    /// On Microsoft Windows, it is possible to limit the access to shared reading or to be fully
    /// exclusive using [`share_mode`].
    ///
    /// On most Unix systems, it is possible to use [`nix::fcntl::flock`]. However, keep in mind
    /// that this provides an **advisory** locking scheme, and that implementations are therefore
    /// required to be co-operative.
    ///
    /// On Linux, it is also possible to mark the file as immutable. See `man 2 ioctl_iflags` and
    /// `man 1 chattr` for more information.
    ///
    /// [`OpenOptionsExt`]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html
    /// [`access_mode`]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.access_mode
    /// [`share_mode`]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.share_mode
    /// [`nix::fcntl::flock`]: https://docs.rs/nix/latest/nix/fcntl/fn.flock.html
    pub unsafe fn with_file(self, file: File, offset: u64) -> Self {
        Self {
            inner: self.inner.with_file(file, offset),
        }
    }

    /// The desired configuration of the mapping. See [`MmapFlags`] for available options.
    pub fn with_flags(self, flags: MmapFlags) -> Self {
        Self {
            inner: self.inner.with_flags(flags),
        }
    }

    /// The desired configuration of the mapping. See [`UnsafeMmapFlags`] for available options.
    ///
    /// Note this function is **unsafe** as the flags that can be passed to this function have
    /// unsafe behavior associated with them.
    pub unsafe fn with_unsafe_flags(self, flags: UnsafeMmapFlags) -> Self {
        Self {
            inner: self.inner.with_unsafe_flags(flags),
        }
    }

    /// Whether this memory mapped should be backed by a specific page size or not.
    pub fn with_page_size(self, page_size: PageSize) -> Self {
        Self {
            inner: self.inner.with_page_size(page_size),
        }
    }

    /// Maps the memory as inaccessible.
    pub fn map_none(self) -> Result<MmapNone, Error> {
        Ok(MmapNone {
            inner: self.inner.map_none()?,
        })
    }

    /// Maps the memory as immutable.
    pub fn map(self) -> Result<Mmap, Error> {
        Ok(Mmap {
            inner: self.inner.map()?,
        })
    }

    /// Maps the memory as executable.
    pub fn map_exec(self) -> Result<Mmap, Error> {
        Ok(Mmap {
            inner: self.inner.map_exec()?,
        })
    }

    /// Maps the memory as mutable.
    pub fn map_mut(self) -> Result<MmapMut, Error> {
        Ok(MmapMut {
            inner: self.inner.map_mut()?,
        })
    }

    /// Maps the memory as executable and mutable. While this may seem useful for self-modifying
    /// code and JIT engines, it is instead recommended to convert between mutable and executable
    /// mappings using [`Mmap::make_mut()`] and [`MmapMut::make_exec()`] instead.
    ///
    /// As it may be tempting to use this function, this function has been marked as **unsafe**.
    /// Make sure to read the text below to understand the complications of this function before
    /// using it. The [`UnsafeMmapFlags::JIT`] flag must be set for this function to succeed.
    ///
    /// RWX pages are an interesting targets to attackers, e.g. for buffer overflow attacks, as RWX
    /// mappings can potentially simplify such attacks. Without RWX mappings, attackers instead
    /// have to resort to return-oriented programming (ROP) gadgets. To prevent buffer overflow
    /// attacks, contemporary CPUs allow pages to be marked as non-executable which is then used by
    /// the operating system to ensure that pages are either marked as writeable or as executable,
    /// but not both. This is also known as W^X.
    ///
    /// While the x86 and x86-64 architectures guarantee cache coherency between the L1 instruction
    /// and the L1 data cache, other architectures such as Arm and AArch64 do not. If the user
    /// modified the pages, then executing the code may result in undefined behavior. To ensure
    /// correct behavior a user has to flush the instruction cache after  modifying and before
    /// executing the page.
    pub unsafe fn map_exec_mut(self) -> Result<MmapMut, Error> {
        Ok(MmapMut {
            inner: self.inner.map_exec_mut()?,
        })
    }
}
