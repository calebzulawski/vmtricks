#[cfg(not(any(unix, windows)))]
compile_error!("This crate only supports Unix- and Windows-family operating systems!");

mod error;
pub use error::SystemError;

pub mod vm;
