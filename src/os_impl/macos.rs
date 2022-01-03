use crate::areas::{MemoryArea, Protection, ShareMode};
use crate::error::Error;
use libc::proc_regionfilename;
use mach2::{
    kern_return::{KERN_INVALID_ADDRESS, KERN_SUCCESS},
    port::mach_port_name_t,
    traps::{mach_task_self, task_for_pid},
    vm::mach_vm_region,
    vm_prot::{VM_PROT_EXECUTE, VM_PROT_READ, VM_PROT_WRITE},
    vm_region::VM_REGION_BASIC_INFO_64,
    vm_region::{vm_region_info_t, vm_region_basic_info_64},
    vm_types::mach_vm_address_t,
};
use nix::unistd::getpid;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::Path;

pub struct MemoryMaps<B> {
    pid: u32,
    task: mach_port_name_t,
    address: mach_vm_address_t,
    marker: PhantomData<B>,
}

impl MemoryMaps<BufReader<File>> {
    pub fn open(pid: Option<u32>) -> Result<Self, Error> {
        let task = unsafe {
            mach_task_self()
        };

        if let Some(pid) = pid {
            let result = unsafe {
                task_for_pid(
                    task,
                    pid as i32,
                    std::mem::transmute(&task),
                )
            };

            if result != KERN_SUCCESS {
                return Err(Error::Mach(result));
            }
        }

        Ok(Self {
            pid: pid.unwrap_or(getpid().as_raw() as _),
            task,
            address: 0,
            marker: PhantomData,
        })
    }
}

impl<B: BufRead> Iterator for MemoryMaps<B> {
    type Item = Result<MemoryArea, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut size = 0;
        let mut info: vm_region_basic_info_64 = unsafe { std::mem::zeroed() };

        let result = unsafe {
            mach_vm_region(
                self.task,
                &mut self.address,
                &mut size,
                VM_REGION_BASIC_INFO_64,
                (&mut info as *mut _) as vm_region_info_t,
                &mut vm_region_basic_info_64::count(),
                &mut 0,
            )
        };

        match result {
            KERN_INVALID_ADDRESS => None,
            KERN_SUCCESS => {
                let start = self.address as usize;
                let end = start + size as usize;
                let range = start..end;

                let mut protection = Protection::empty();

                if info.protection & VM_PROT_READ == VM_PROT_READ {
                    protection |= Protection::READ;
                }

                if info.protection & VM_PROT_WRITE == VM_PROT_WRITE {
                    protection |= Protection::WRITE;
                }

                if info.protection & VM_PROT_EXECUTE == VM_PROT_EXECUTE {
                    protection |= Protection::EXECUTE;
                }

                let share_mode = if info.shared != 0 {
                    ShareMode::Shared
                } else {
                    ShareMode::Private
                };

                let mut bytes = [0u8; libc::PATH_MAX as _];

                let result = unsafe {
                    proc_regionfilename(
                        self.pid as _,
                        self.address,
                        bytes.as_mut_ptr() as _,
                        bytes.len() as _,
                    )
                };

                let path = if result == 0 {
                    None
                } else {
                    let path = match std::str::from_utf8(&bytes[..result as usize]) {
                        Ok(path) => path,
                        Err(e) => return Some(Err(Error::Utf8(e))),
                    };

                    Some((Path::new(path).to_path_buf(), info.offset as u64))
                };

                self.address = self.address.saturating_add(size);

                Some(Ok(MemoryArea {
                    range,
                    protection,
                    share_mode,
                    path,
                }))
            }
            _ => Some(Err(Error::Mach(result))),
        }
    }
}
