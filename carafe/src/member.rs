//! Fields and methods (JVMS 4.5 and 4.6).

use crate::attribute::{Attribute, Code, read_attributes, take_one};
use crate::descriptor::{FieldType, MethodDescriptor};
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::pool::ConstantPool;
use crate::reader::Reader;

// @listing field
/// A field, with its descriptor parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub access_flags: AccessFlags,
    pub name: String,
    pub descriptor: String,
    pub ty: FieldType,
    /// The index of the constant a `static final` field starts with.
    pub constant_value: Option<u16>,
    pub attributes: Vec<Attribute>,
}

/// A method, with its descriptor parsed and its `Code` attribute read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Method {
    pub access_flags: AccessFlags,
    pub name: String,
    pub descriptor: String,
    pub ty: MethodDescriptor,
    /// `None` for an `abstract` or a `native` method, which has no bytecode.
    pub code: Option<Code>,
    pub attributes: Vec<Attribute>,
}

/// What fields and methods have in common: flags, a name, a descriptor and
/// attributes (JVMS 4.5 and 4.6 give them the same layout).
fn read_member(
    r: &mut Reader,
    pool: &ConstantPool,
) -> Result<(AccessFlags, String, String, Vec<Attribute>)> {
    let access_flags = AccessFlags(r.u2()?);
    let name = pool.utf8(r.u2()?)?.to_string();
    let descriptor = pool.utf8(r.u2()?)?.to_string();
    let attributes = read_attributes(r, pool)?;
    Ok((access_flags, name, descriptor, attributes))
}
// @end

impl Field {
    // @listing field-read
    pub(crate) fn read(r: &mut Reader, pool: &ConstantPool) -> Result<Field> {
        let (access_flags, name, descriptor, mut attributes) = read_member(r, pool)?;
        let ty = FieldType::parse(&descriptor)?;
        // A JVM ignores the attribute on an instance field (JVMS 4.7.2).
        let attribute = if access_flags.has(AccessFlags::STATIC) {
            take_one(&mut attributes, "ConstantValue")?
        } else {
            None
        };
        let constant_value = match attribute {
            None => None,
            Some(Attribute { info, .. }) => {
                let Ok(index) = <[u8; 2]>::try_from(&info[..]) else {
                    let m =
                        format!("a ConstantValue attribute of {} bytes", info.len());
                    return Err(Error::ClassFormat(m));
                };
                let index = u16::from_be_bytes(index);
                pool.get(index)?;
                Some(index)
            }
        };
        Ok(Field { access_flags, name, descriptor, ty, constant_value, attributes })
    }
    // @end
}

impl Method {
    // @listing method-read
    pub(crate) fn read(r: &mut Reader, pool: &ConstantPool) -> Result<Method> {
        let (access_flags, name, descriptor, mut attributes) = read_member(r, pool)?;
        let ty = MethodDescriptor::parse(&descriptor)?;
        let code = match take_one(&mut attributes, "Code")? {
            Some(a) => Some(Code::parse(&a.info, pool)?),
            None => None,
        };
        let method = Method { access_flags, name, descriptor, ty, code, attributes };
        method.check()?;
        Ok(method)
    }

    /// The rules of JVMS 4.3.3 and 4.7.3 that need the whole method.
    fn check(&self) -> Result<()> {
        let bodiless =
            self.access_flags.has(AccessFlags::ABSTRACT | AccessFlags::NATIVE);
        let problem = if self.arg_slots() > 255 {
            "takes more than 255 slots of arguments"
        } else if !bodiless && self.code.is_none() {
            "has no Code attribute"
        } else if bodiless && self.code.is_some() {
            "is abstract or native, but has a Code attribute"
        } else {
            return Ok(());
        };
        let m = format!("the method {}{} {problem}", self.name, self.descriptor);
        Err(Error::ClassFormat(m))
    }
    // @end

    // @listing arg-slots
    /// The number of local variable slots the arguments fill when the method
    /// is called: `this`, unless the method is static, then the parameters.
    pub fn arg_slots(&self) -> usize {
        let this = usize::from(!self.access_flags.has(AccessFlags::STATIC));
        this + self.ty.param_slots()
    }
    // @end
}
