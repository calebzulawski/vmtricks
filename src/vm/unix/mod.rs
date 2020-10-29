mod shm_open_anonymous;
use nix::Result;

pub fn allocation_granularity() -> usize {
    use nix::unistd::{sysconf, SysconfVar};
    sysconf(SysconfVar::PAGE_SIZE).unwrap().unwrap() as usize
}

pub struct MirroredAllocation<T> {
    ptr: *mut T,
    size: usize,
}

impl<T> MirroredAllocation<T> {
    pub fn new(size: usize) -> Result<Self> {
        use nix::{
            sys::mman::{mmap, MapFlags, ProtFlags},
            unistd::ftruncate,
        };
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

            // Create the shared memory file
            let fd = shm_open_anonymous::shm_open_anonymous()?;
            ftruncate(fd.as_fd(), double_size as i64)?;

            // Create the memory region
            let mirrored = Self {
                ptr: unsafe {
                    mmap(
                        std::ptr::null_mut(),
                        double_size,
                        ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                        MapFlags::MAP_SHARED,
                        fd.as_fd(),
                        0,
                    )? as *mut T
                },
                size: size / core::mem::size_of::<T>(),
            };
            assert_eq!(
                (mirrored.as_mut_ptr() as usize) % core::mem::align_of::<T>(),
                0
            );

            // Remap the mirrored region
            unsafe {
                mmap(
                    (mirrored.as_mut_ptr()).add(mirrored.len()) as *mut _,
                    size,
                    ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                    MapFlags::MAP_SHARED | MapFlags::MAP_FIXED,
                    fd.as_fd(),
                    0,
                )?;
            }

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

impl<T> Drop for MirroredAllocation<T> {
    fn drop(&mut self) {
        if !self.as_mut_ptr().is_null() {
            unsafe {
                let _ = nix::sys::mman::munmap(
                    self.ptr as *mut _,
                    2 * self.size * core::mem::size_of::<T>(),
                );
            }
        }
    }
}
