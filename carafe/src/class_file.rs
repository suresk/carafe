//! A class file as a whole (JVMS 4.1).

use crate::attribute::{Attribute, read_attributes};
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::member::{Field, Method};
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
/// A parsed class file (JVMS 4.1). Class names are in internal form,
/// `java/lang/Object`, and `super_class` is `None` only for `Object` itself
/// and for a module descriptor.
#[derive(Debug)]
pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: ConstantPool,
    pub access_flags: AccessFlags,
    pub this_class: String,
    pub super_class: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
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
        let pool = ConstantPool::read(&mut r)?;
        let access_flags = AccessFlags(r.u2()?);
        let this_class = pool.class_name(r.u2()?)?.to_string();
        let super_class = match r.u2()? {
            0 => None,
            index => Some(pool.class_name(index)?.to_string()),
        };
        let mut interfaces = Vec::new();
        for _ in 0..r.u2()? {
            interfaces.push(pool.class_name(r.u2()?)?.to_string());
        }
        let fields = (0..r.u2()?).map(|_| Field::read(&mut r, &pool));
        let fields = fields.collect::<Result<_>>()?;
        let methods = (0..r.u2()?).map(|_| Method::read(&mut r, &pool));
        let methods = methods.collect::<Result<_>>()?;
        let attributes = read_attributes(&mut r, &pool)?;
        let class = ClassFile {
            minor_version,
            major_version,
            constant_pool: pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        };
        class.check(r.left())?;
        Ok(class)
    }

    /// The rules of JVMS 4.1 that need the whole file. `left` is the number
    /// of bytes after the last attribute.
    fn check(&self, left: usize) -> Result<()> {
        let root = self.this_class == "java/lang/Object"
            || self.access_flags.has(AccessFlags::MODULE);
        let problem = if left > 0 {
            format!("has {left} bytes after its last attribute")
        } else if self.super_class.is_none() && !root {
            "has no superclass".to_string()
        } else {
            return Ok(());
        };
        let m = format!("the class file of {} {problem}", self.this_class);
        Err(Error::ClassFormat(m))
    }

    /// The method with this name and descriptor, if the class declares one.
    pub fn method(&self, name: &str, descriptor: &str) -> Option<&Method> {
        self.methods.iter().find(|m| m.name == name && m.descriptor == descriptor)
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
