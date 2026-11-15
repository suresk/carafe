//! A class file as a whole (JVMS 4.1).

use crate::error::{Error, Result};
use crate::pool::ConstantPool;
use crate::reader::Reader;

// @listing versions
/// The four bytes every class file starts with.
pub const MAGIC: u32 = 0xcafe_babe;

/// The oldest class file version carafe runs: Java 7. Older class files may
/// contain the `jsr` and `ret` instructions, which version 51 removed.
pub const MIN_MAJOR: u16 = 51;

/// The newest: Java 25, the release carafe is pinned to.
pub const MAX_MAJOR: u16 = 69;
// @end

// @listing class-file
/// A parsed class file. So far: the version and the constant pool.
#[derive(Debug)]
pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: ConstantPool,
}
// @end

impl ClassFile {
    // @listing parse
    pub fn parse(bytes: &[u8]) -> Result<ClassFile> {
        let mut r = Reader::new(bytes);
        let magic = r.u4()?;
        if magic != MAGIC {
            let m = format!("the file starts with {magic:#010x}, not 0xcafebabe");
            return Err(Error::ClassFormat(m));
        }
        let minor_version = r.u2()?;
        let major_version = r.u2()?;
        check_version(major_version, minor_version)?;
        let constant_pool = ConstantPool::read(&mut r)?;
        Ok(ClassFile { minor_version, major_version, constant_pool })
    }
    // @end
}

// @listing check-version
fn check_version(major: u16, minor: u16) -> Result<()> {
    let unsupported = |why: String| {
        let m = format!("class file version {major}.{minor} {why}");
        Err(Error::UnsupportedVersion(m))
    };
    if !(MIN_MAJOR..=MAX_MAJOR).contains(&major) {
        return unsupported(format!(
            "is outside the versions carafe runs, {MIN_MAJOR}.0 to {MAX_MAJOR}.0"
        ));
    }
    match (major >= 56, minor) {
        (true, 0xffff) => unsupported("uses preview features".to_string()),
        (true, 1..) => unsupported("has a minor version that is not 0".to_string()),
        _ => Ok(()),
    }
}
// @end
