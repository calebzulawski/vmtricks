#[cfg(unix)]
pub mod posix {
    use libc::{c_char, c_long};

    pub fn shm_open_anonymous() -> c_long {
        #[cfg(target_os = "linux")]
        unsafe {
            use libc::{syscall, SYS_memfd_create, MFD_CLOEXEC};
            syscall(
                SYS_memfd_create,
                b"/shm-vmtricks\0".as_ptr() as *const c_char,
                MFD_CLOEXEC,
            )
        }

        #[cfg(target_os = "freebsd")]
        unsafe {
            use libc::{shm_open, O_RDWR, SHM_ANON};
            shm_open(SHM_ANON, O_RDWR, 0)
        }

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        unsafe {
            use libc::{shm_open, shm_unlink, O_CREAT, O_EXCL, O_NOFOLLOW, O_RDWR};
            use once_cell::sync::Lazy;
            use std::sync::Mutex;
            static MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
            const FILENAME: *const c_char = b"/shm-vmtricks\0".as_ptr() as *const c_char;
            let guard = MUTEX.lock().unwrap();
            let fd = shm_open(FILENAME, O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW, 0600) as c_long;
            core::mem::drop(guard);
            if fd != -1 {
                assert!(shm_unlink(FILENAME) != -1);
            }
            fd
        }
    }

    #[cfg(test)]
    mod test {
        #[test]
        fn shm_open_anonymous() {
            let fd = super::shm_open_anonymous();
            assert!(fd != -1);
            unsafe { assert!(libc::close(fd as _) != -1) };
        }
    }
}

pub fn allocation_granularity() -> usize {
    #[cfg(unix)]
    unsafe {
        use libc::{sysconf, _SC_PAGESIZE};
        sysconf(_SC_PAGESIZE) as usize
    }
}

pub fn allocate(iter: impl Iterator<Item = usize> + Clone) -> *mut u8 {
    let total_blocks = iter
        .clone()
        .fold(0usize, |blocks, segment| blocks + segment);

    let fd = posix::shm_open_anonymous();
    assert!(fd != -1);

    core::ptr::null_mut()
}
