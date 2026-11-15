//! One error type for the whole crate.

use crate::heap::Ref;
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
    // @hide
    /// Code broke a rule that the JVM's verifier enforces: `VerifyError`.
    Verify(String),
    /// A Java exception or error that the VM throws, such as
    /// `java/lang/ArithmeticException`, until Chapter 16 makes it an object.
    Exception { class: &'static str, message: Option<String> },
    /// Something carafe does not implement yet.
    Unsupported(String),
    /// A Java exception object that no handler caught: its class, as
    /// `java/lang/IllegalStateException`, and its message.
    Thrown { exception: Ref, class: String, message: Option<String> },
    // @show
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
            // @hide
            Error::Verify(m) => write!(f, "VerifyError: {m}"),
            Error::Exception { class, message } => {
                write!(f, "{}", class.replace('/', "."))?;
                match message {
                    Some(m) => write!(f, ": {m}"),
                    None => Ok(()),
                }
            }
            Error::Thrown { class, message, .. } => {
                write!(f, "{}", class.replace('/', "."))?;
                match message {
                    Some(m) => write!(f, ": {m}"),
                    None => Ok(()),
                }
            }
            Error::Unsupported(m) => write!(f, "carafe cannot run this yet: {m}"),
            // @show
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
