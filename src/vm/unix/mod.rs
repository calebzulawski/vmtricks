mod shm_open_anonymous;
use nix::Result;
use std::os::unix::io::RawFd;

pub fn allocation_granularity() -> usize {
    use nix::unistd::{sysconf, SysconfVar};
    sysconf(SysconfVar::PAGE_SIZE).unwrap().unwrap() as usize
}

unsafe fn allocate_mirrored(fd: RawFd, size: usize) -> Result<*mut std::ffi::c_void> {
    use nix::{
        sys::mman::{mmap, munmap, MapFlags, ProtFlags},
        unistd::ftruncate,
    };
    let double_size = size.checked_mul(2).unwrap();
    ftruncate(fd, double_size as i64)?;
    let ptr = {
        let ptr = mmap(
            std::ptr::null_mut(),
            double_size,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            fd,
            0,
        )?;
        mmap(
            (ptr as *mut u8).add(size) as *mut _,
            size,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED | MapFlags::MAP_FIXED,
            fd,
            0,
        )
        .map_err(|err| {
            let _ = munmap(ptr, double_size);
            err
        })?;
        ptr
    };
    Ok(ptr)
}

pub struct MirroredAllocation<T> {
    fd: RawFd,
    ptr: *mut T,
    size: usize,
}

impl<T> MirroredAllocation<T> {
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 {
            Ok(Self {
                fd: -1,
                ptr: std::ptr::null_mut(),
                size: 0,
            })
        } else {
            use num_integer::{div_ceil, lcm};
            let granularity = lcm(allocation_granularity(), core::mem::size_of::<T>());
            let size = div_ceil(size, granularity)
                .checked_mul(granularity)
                .unwrap();
            let fd = shm_open_anonymous::shm_open_anonymous()?;
            let ptr = unsafe { allocate_mirrored(fd, size) }.map_err(|err| {
                let _ = nix::unistd::close(fd);
                err
            })? as *mut _;
            assert_eq!((ptr as usize) % core::mem::align_of::<T>(), 0);
            Ok(Self { fd, ptr, size })
        }
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

impl<T> Drop for MirroredAllocation<T> {
    fn drop(&mut self) {
        use nix::{sys::mman::munmap, unistd::close};
        if self.fd != -1 {
            unsafe {
                let _ = munmap(self.ptr as *mut _, self.size * core::mem::size_of::<T>());
            }
            let _ = close(self.fd);
        }
    }
}
