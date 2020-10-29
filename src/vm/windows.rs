use crate::error::windows::Error;
use winapi::um::winnt::HANDLE;

pub fn allocation_granularity() -> usize {
    use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
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
        use winapi::um::handleapi::CloseHandle;
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
    use std::convert::TryInto;
    use winapi::{
        shared::minwindef::DWORD,
        um::{
            handleapi::INVALID_HANDLE_VALUE,
            winbase::CreateFileMappingA,
            winnt::{PAGE_READWRITE, SEC_COMMIT},
        },
    };
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

pub struct MirroredAllocation<T> {
    ptr: *mut T,
    size: usize,
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

            let handle = create_file_mapping(size);

            let mirrored = Self {
                ptr: std::ptr::null_mut(),
                size: 0,
            };

            assert_eq!(
                (mirrored.as_mut_ptr() as usize) % core::mem::align_of::<T>(),
                0
            );

            Ok(mirrored)
        }
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.size
    }
}
