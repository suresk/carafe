//! Attributes (JVMS 4.7), and the one that holds a method's bytecode.

use crate::error::{Error, Result};
use crate::pool::ConstantPool;
use crate::reader::Reader;

// @listing attribute
/// An attribute as the class file stores it: a name and uninterpreted bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub info: Vec<u8>,
}
// @end

// @listing read-attributes
/// Reads an attribute table: a `u2` count, then the attributes.
pub(crate) fn read_attributes(
    r: &mut Reader,
    pool: &ConstantPool,
) -> Result<Vec<Attribute>> {
    let count = r.u2()?;
    let mut attributes = Vec::with_capacity(count.into());
    for _ in 0..count {
        let name = pool.utf8(r.u2()?)?.to_string();
        let length = r.u4()?;
        let info = r.take(length as usize)?.to_vec();
        attributes.push(Attribute { name, info });
    }
    Ok(attributes)
}
// @end

// @listing take-one
/// Removes the attribute called `name` from `attributes` and returns it, or
/// `None` if there is none. More than one is a `ClassFormat` error.
pub(crate) fn take_one(
    attributes: &mut Vec<Attribute>,
    name: &str,
) -> Result<Option<Attribute>> {
    let (mut found, rest): (Vec<_>, Vec<_>) =
        std::mem::take(attributes).into_iter().partition(|a| a.name == name);
    *attributes = rest;
    if found.len() > 1 {
        let m = format!("{} {name} attributes where one is allowed", found.len());
        return Err(Error::ClassFormat(m));
    }
    Ok(found.pop())
}
// @end

// @listing code
/// A method's bytecode and what the interpreter needs to run it (JVMS 4.7.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub bytecode: Vec<u8>,
    pub handlers: Vec<Handler>,
    pub attributes: Vec<Attribute>,
}

/// One entry of the exception table: if an exception of class `catch_type`
/// is thrown in `start_pc..end_pc`, execution continues at `handler_pc`.
/// A `catch_type` of 0 catches everything, as `finally` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handler {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: u16,
}

impl Code {
    /// Parses the contents of a `Code` attribute.
    pub(crate) fn parse(info: &[u8], pool: &ConstantPool) -> Result<Code> {
        let mut r = Reader::new(info);
        let max_stack = r.u2()?;
        let max_locals = r.u2()?;
        let length = r.u4()?;
        if !(1..=65535).contains(&length) {
            let m = format!("a code length of {length}, not 1 to 65535");
            return Err(Error::ClassFormat(m));
        }
        let bytecode = r.take(length as usize)?.to_vec();
        let count = r.u2()?;
        let mut handlers = Vec::with_capacity(count.into());
        for _ in 0..count {
            handlers.push(Handler::read(&mut r, pool, bytecode.len())?);
        }
        let attributes = read_attributes(&mut r, pool)?;
        if r.left() > 0 {
            let m = format!("{} bytes left over in a Code attribute", r.left());
            return Err(Error::ClassFormat(m));
        }
        Ok(Code { max_stack, max_locals, bytecode, handlers, attributes })
    }
}

impl Handler {
    fn read(r: &mut Reader, pool: &ConstantPool, length: usize) -> Result<Handler> {
        let [start_pc, end_pc, handler_pc, catch_type] =
            [r.u2()?, r.u2()?, r.u2()?, r.u2()?];
        let inside = |pc: u16| usize::from(pc) < length;
        if start_pc >= end_pc || usize::from(end_pc) > length || !inside(handler_pc) {
            let m = format!(
                "the handler {start_pc}..{end_pc} -> {handler_pc} \
                 is outside the {length} bytes of code"
            );
            return Err(Error::ClassFormat(m));
        }
        if catch_type != 0 {
            pool.class_name(catch_type)?;
        }
        Ok(Handler { start_pc, end_pc, handler_pc, catch_type })
    }
}
// @end
