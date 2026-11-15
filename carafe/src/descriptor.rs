//! Field and method descriptors (JVMS 4.3).

use crate::error::{Error, Result};
use std::fmt;

// @listing field-type
/// The type of a field, a parameter or a return value (JVMS 4.3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
    /// A class or interface, by its name in internal form: `java/lang/String`.
    Object(String),
    /// An array, by the type of its elements.
    Array(Box<FieldType>),
}

/// The parameter types and return type of a method (JVMS 4.3.3). `ret` is
/// `None` for `void`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodDescriptor {
    pub params: Vec<FieldType>,
    pub ret: Option<FieldType>,
}
// @end

// @listing descriptor-parse
impl FieldType {
    /// Parses a field descriptor, such as `[Ljava/lang/String;`.
    pub fn parse(text: &str) -> Result<FieldType> {
        match field_type(text) {
            Some((ty, "")) => Ok(ty),
            _ => Err(malformed(text)),
        }
    }

    /// The number of local variable slots a value of this type occupies.
    pub fn slots(&self) -> usize {
        match self {
            FieldType::Long | FieldType::Double => 2,
            _ => 1,
        }
    }
}

impl MethodDescriptor {
    /// Parses a method descriptor, such as `([Ljava/lang/String;)V`.
    pub fn parse(text: &str) -> Result<MethodDescriptor> {
        let mut rest = text.strip_prefix('(').ok_or_else(|| malformed(text))?;
        let mut params = Vec::new();
        while let Some((ty, after)) = field_type(rest) {
            params.push(ty);
            rest = after;
        }
        let ret = match rest.strip_prefix(')') {
            Some("V") => None,
            Some(ret) => Some(FieldType::parse(ret).map_err(|_| malformed(text))?),
            None => return Err(malformed(text)),
        };
        Ok(MethodDescriptor { params, ret })
    }

    /// The number of slots the parameters occupy, not counting `this`.
    pub fn param_slots(&self) -> usize {
        self.params.iter().map(FieldType::slots).sum()
    }
}
// @end

// @listing parse-field-type
/// Parses one field type from the start of `text` and returns it with the
/// text that follows, or `None` if `text` doesn't start with one.
fn field_type(text: &str) -> Option<(FieldType, &str)> {
    let rest = text.trim_start_matches('[');
    let dimensions = text.len() - rest.len();
    if dimensions > 255 {
        return None; // JVMS 4.3.2
    }
    let (base, rest) = match rest.as_bytes().first()? {
        b'B' => (FieldType::Byte, &rest[1..]),
        b'C' => (FieldType::Char, &rest[1..]),
        b'D' => (FieldType::Double, &rest[1..]),
        b'F' => (FieldType::Float, &rest[1..]),
        b'I' => (FieldType::Int, &rest[1..]),
        b'J' => (FieldType::Long, &rest[1..]),
        b'S' => (FieldType::Short, &rest[1..]),
        b'Z' => (FieldType::Boolean, &rest[1..]),
        b'L' => {
            let (name, rest) = rest[1..].split_once(';')?;
            if name.split('/').any(str::is_empty) || name.contains(['.', '[']) {
                return None; // not a name in internal form (JVMS 4.2.1)
            }
            (FieldType::Object(name.to_string()), rest)
        }
        _ => return None,
    };
    let ty = (0..dimensions).fold(base, |ty, _| FieldType::Array(Box::new(ty)));
    Some((ty, rest))
}

fn malformed(text: &str) -> Error {
    Error::ClassFormat(format!("malformed descriptor {text:?}"))
}
// @end

// @listing field-type-display
/// A type as Java source writes it: `int`, `java.lang.String[]`.
impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            FieldType::Byte => "byte",
            FieldType::Char => "char",
            FieldType::Double => "double",
            FieldType::Float => "float",
            FieldType::Int => "int",
            FieldType::Long => "long",
            FieldType::Short => "short",
            FieldType::Boolean => "boolean",
            FieldType::Object(name) => return f.write_str(&name.replace('/', ".")),
            FieldType::Array(element) => return write!(f, "{element}[]"),
        };
        f.write_str(name)
    }
}
// @end

impl FieldType {
    // @listing descriptor
    /// The type as a descriptor: `I`, `Ljava/lang/String;`, `[[J`. An array
    /// class's name is its descriptor.
    pub fn descriptor(&self) -> String {
        match self {
            FieldType::Byte => "B".into(),
            FieldType::Char => "C".into(),
            FieldType::Double => "D".into(),
            FieldType::Float => "F".into(),
            FieldType::Int => "I".into(),
            FieldType::Long => "J".into(),
            FieldType::Short => "S".into(),
            FieldType::Boolean => "Z".into(),
            FieldType::Object(name) => format!("L{name};"),
            FieldType::Array(element) => format!("[{}", element.descriptor()),
        }
    }
    // @end
}
