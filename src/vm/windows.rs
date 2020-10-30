use super::MirroredAllocation;
use crate::error::windows::Error;
use std::convert::TryInto;
use winapi::{
    ctypes::c_void,
    shared::minwindef::DWORD,
    um::{
        handleapi::CloseHandle,
        handleapi::INVALID_HANDLE_VALUE,
        memoryapi::{
            MapViewOfFileEx, UnmapViewOfFile, VirtualAlloc, VirtualFree, FILE_MAP_ALL_ACCESS,
        },
        sysinfoapi::{GetSystemInfo, SYSTEM_INFO},
        winbase::CreateFileMappingA,
        winnt::{HANDLE, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE, SEC_COMMIT},
    },
};

pub fn allocation_granularity() -> usize {
    let system_info = unsafe {
        let mut system_info = std::mem::MaybeUninit::<SYSTEM_INFO>::uninit();
        GetSystemInfo(system_info.as_mut_ptr());
        system_info.assume_init()
    };
    system_info.dwAllocationGranularity as usize
}

struct FileHandle(HANDLE);

impl Drop for FileHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

impl FileHandle {
    fn as_handle(&self) -> HANDLE {
        self.0
    }
}

fn create_file_mapping(size: usize) -> Result<FileHandle, Error> {
    let handle = unsafe {
        CreateFileMappingA(
            INVALID_HANDLE_VALUE,
            std::ptr::null_mut(),
            PAGE_READWRITE | SEC_COMMIT,
            size.checked_shr(std::mem::size_of::<DWORD>() as u32 * 8)
                .unwrap_or(0)
                .try_into()
                .unwrap(),
            size.try_into().unwrap(),
            std::ptr::null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        Err(Error::last())
    } else {
        Ok(FileHandle(handle))
    }
}

fn reserve_memory(size: usize) -> Result<*mut c_void, Error> {
    unsafe {
        let address = VirtualAlloc(std::ptr::null_mut(), size, MEM_RESERVE, PAGE_NOACCESS);
        if address.is_null() || VirtualFree(address, 0, MEM_RELEASE) == 0 {
            Err(Error::last())
        } else {
            Ok(address)
        }
    }
}

unsafe fn map_view_of_file(handle: HANDLE, size: usize, address: *mut c_void) -> Result<(), Error> {
    let address = MapViewOfFileEx(handle, FILE_MAP_ALL_ACCESS, 0, 0, size, address);
    if address.is_null() {
        Err(Error::last())
    } else {
        Ok(())
    }
}

unsafe fn unmap_view_of_file(address: *mut c_void) -> Result<(), Error> {
    if UnmapViewOfFile(address) == 0 {
        Err(Error::last())
    } else {
        Ok(())
    }
}

impl<T> Drop for MirroredAllocation<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                unmap_view_of_file(self.ptr as *mut c_void).unwrap();
                unmap_view_of_file(self.ptr.add(self.size) as *mut c_void).unwrap();
            }
        }
    }
}

impl<T> MirroredAllocation<T> {
    pub fn new(size: usize) -> Result<Self, Error> {
        use num_integer::{div_ceil, lcm};
        if size == 0 {
            Ok(Self {
                ptr: std::ptr::null_mut(),
                size: 0,
            })
        } else {
            // Determine the appropriate size.  Must be a multiple of page size and type size.
            let granularity = lcm(allocation_granularity(), core::mem::size_of::<T>());
            let size = div_ceil(size, granularity)
                .checked_mul(granularity)
                .unwrap();
            let double_size = size.checked_mul(2).unwrap();

            // Create the underlying file mapping
            let handle = create_file_mapping(size)?;

            // Try to create the mappings
            let mut tries = 0;
            const MAX_TRIES: usize = 5;
            let ptr = loop {
                tries += 1;
                let ptr = reserve_memory(double_size)?;
                unsafe {
                    println!("{}", size);
                    println!("{:?}", ptr);
                    println!("{:?}", ptr.add(size));
                    if let Err(err) = map_view_of_file(handle.as_handle(), size, ptr) {
                        if tries == MAX_TRIES {
                            break Err(err);
                        } else {
                            continue;
                        }
                    }
                    if let Err(err) = map_view_of_file(handle.as_handle(), size, ptr.add(size)) {
                        if tries == MAX_TRIES {
                            unmap_view_of_file(ptr).unwrap();
                            break Err(err);
                        } else {
                            continue;
                        }
                    }
                }
                break Ok(ptr as *mut T);
            }?;

            let mirrored = Self {
                ptr,
                size: size / std::mem::size_of::<T>(),
            };

            assert_eq!(
                (mirrored.as_mut_ptr() as usize) % core::mem::align_of::<T>(),
                0
            );

            Ok(mirrored)
        }
    }
}
