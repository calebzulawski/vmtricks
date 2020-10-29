//! Virtual memory utilities

#[cfg(unix)]
mod unix;

mod implementation {
    #[cfg(unix)]
    pub use super::unix::MirroredAllocation;
}

pub struct MirroredAllocation<T>(implementation::MirroredAllocation<T>);

impl<T> MirroredAllocation<T> {
    pub fn new(size: usize) -> Result<Self, crate::error::SystemError> {
        Ok(Self(implementation::MirroredAllocation::new(size)?))
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.0.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Default for MirroredAllocation<T> {
    fn default() -> Self {
        Self::new(0).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default() {
        let mirrored = MirroredAllocation::<u8>::default();
        assert!(mirrored.as_mut_ptr().is_null());
        assert!(mirrored.is_empty());
    }

    #[test]
    fn assorted_sizes() {
        fn test_impl(size: usize) {
            let mirrored = MirroredAllocation::<u8>::new(size).unwrap();
            assert!(!mirrored.as_mut_ptr().is_null());
            assert!(mirrored.len() >= size);
        }

        test_impl(100);
        test_impl(4000);
        test_impl(4096);
        test_impl(4100);
        test_impl(65000);
        test_impl(65536);
        test_impl(66000);
        test_impl(1000000);
    }
}
