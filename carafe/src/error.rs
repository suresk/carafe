//! One error type for the whole crate.

use std::fmt;
use std::io;

// @listing error
#[derive(Debug)]
pub enum Error {
    /// Reading a file, or running a JDK tool, returned an I/O error.
    Io(io::Error),
    /// A class file is malformed: the JVM's `ClassFormatError`.
    ClassFormat(String),
    /// A class file's version is one carafe does not run:
    /// `UnsupportedClassVersionError`.
    UnsupportedVersion(String),
    /// The JDK is missing, is not the pinned release, or a tool in it failed.
    Jdk(String),
}

pub type Result<T> = std::result::Result<T, Error>;
// @end

// @listing error-display
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::ClassFormat(m) => write!(f, "ClassFormatError: {m}"),
            Error::UnsupportedVersion(m) => {
                write!(f, "UnsupportedClassVersionError: {m}")
            }
            Error::Jdk(m) => write!(f, "JDK: {m}"),
        }
    }
}
// @end

// @listing error-impls
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}
// @end
