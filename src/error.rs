#[derive(Debug)]
pub struct SystemError(#[cfg(unix)] nix::Error);

impl std::error::Error for SystemError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Display for SystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

#[cfg(unix)]
impl From<nix::Error> for SystemError {
    fn from(e: nix::Error) -> Self {
        Self(e)
    }
}
