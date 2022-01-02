//! This module implements the error type used throughout this crate.

use crate::UnsafeMmapFlags;
use thiserror::Error;

/// The error type.
#[derive(Debug, Error)]
pub enum Error {
    /// The following set of unsafe flags must be set to call this function.
    #[error("{0:?} must be set")]
    UnsafeFlagNeeded(UnsafeMmapFlags),

    /// Represents [`std::io::Error`].
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Represents [`std::num::ParseIntError`].
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// Represents [`std::str::Utf8Error`].
    #[error(transparent)]
    Utf8(std::str::Utf8Error),

    #[cfg(unix)]
    /// Represents [`nix::Error`].
    #[error(transparent)]
    Nix(#[from] nix::Error),

    #[cfg(unix)]
    /// Represents [`sysctl::SysctlError`].
    #[error(transparent)]
    Sysctl(#[from] sysctl::SysctlError),
}
