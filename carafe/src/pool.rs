//! The constant pool: every name, string and number a class file refers to.

use crate::error::{Error, Result};
use crate::reader::Reader;
use crate::utf8::decode_modified_utf8;
use std::fmt;

// @listing constant
/// One entry of the constant pool (JVMS 4.4). The `u16` fields are indices
/// of other entries in the same pool. `Unusable` stands for index 0 and the
/// index after each `Long` and `Double`. `Utf16` is a `Utf8` entry with a
/// surrogate that has no partner, which a Java string may hold and a Rust
/// `String` may not.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Unusable,
    Utf8(String),
    Utf16(Vec<u16>),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class { name: u16 },
    String { utf8: u16 },
    Fieldref { class: u16, name_and_type: u16 },
    Methodref { class: u16, name_and_type: u16 },
    InterfaceMethodref { class: u16, name_and_type: u16 },
    NameAndType { name: u16, descriptor: u16 },
    MethodHandle { kind: u8, reference: u16 },
    MethodType { descriptor: u16 },
    Dynamic { bootstrap: u16, name_and_type: u16 },
    InvokeDynamic { bootstrap: u16, name_and_type: u16 },
    Module { name: u16 },
    Package { name: u16 },
}
// @end

impl Constant {
    // @listing describe
    /// The tag's name and the operands, as `javap -v` prints them.
    pub fn describe(&self) -> (&'static str, String) {
        match self {
            Constant::Unusable => ("Unusable", String::new()),
            Constant::Utf8(s) => {
                ("Utf8", escape(&s.encode_utf16().collect::<Vec<_>>()))
            }
            Constant::Utf16(units) => ("Utf8", escape(units)),
            Constant::Integer(v) => ("Integer", v.to_string()),
            Constant::Float(v) => ("Float", format!("{v:?}f")),
            Constant::Long(v) => ("Long", format!("{v}l")),
            Constant::Double(v) => ("Double", format!("{v:?}d")),
            Constant::Class { name } => ("Class", format!("#{name}")),
            Constant::String { utf8 } => ("String", format!("#{utf8}")),
            Constant::Fieldref { class: c, name_and_type: n } => {
                ("Fieldref", format!("#{c}.#{n}"))
            }
            Constant::Methodref { class: c, name_and_type: n } => {
                ("Methodref", format!("#{c}.#{n}"))
            }
            Constant::InterfaceMethodref { class: c, name_and_type: n } => {
                ("InterfaceMethodref", format!("#{c}.#{n}"))
            }
            Constant::NameAndType { name: n, descriptor: d } => {
                ("NameAndType", format!("#{n}:#{d}"))
            }
            Constant::MethodHandle { kind, reference } => {
                ("MethodHandle", format!("{kind}:#{reference}"))
            }
            Constant::MethodType { descriptor } => {
                ("MethodType", format!("#{descriptor}"))
            }
            Constant::Dynamic { bootstrap: b, name_and_type: n } => {
                ("Dynamic", format!("#{b}:#{n}"))
            }
            Constant::InvokeDynamic { bootstrap: b, name_and_type: n } => {
                ("InvokeDynamic", format!("#{b}:#{n}"))
            }
            Constant::Module { name } => ("Module", format!("#{name}")),
            Constant::Package { name } => ("Package", format!("#{name}")),
        }
    }
    // @end

    // @listing read-constant
    /// Reads one entry: a one-byte tag, then the fields that tag has.
    fn read(r: &mut Reader) -> Result<Constant> {
        let at = r.offset();
        Ok(match r.u1()? {
            1 => {
                let len = r.u2()? as usize;
                text(decode_modified_utf8(r.take(len)?)?)
            }
            3 => Constant::Integer(r.u4()? as i32),
            4 => Constant::Float(f32::from_bits(r.u4()?)),
            5 => Constant::Long(eight(r)? as i64),
            6 => Constant::Double(f64::from_bits(eight(r)?)),
            7 => Constant::Class { name: r.u2()? },
            8 => Constant::String { utf8: r.u2()? },
            9 => Constant::Fieldref { class: r.u2()?, name_and_type: r.u2()? },
            10 => Constant::Methodref { class: r.u2()?, name_and_type: r.u2()? },
            11 => Constant::InterfaceMethodref {
                class: r.u2()?,
                name_and_type: r.u2()?,
            },
            12 => Constant::NameAndType { name: r.u2()?, descriptor: r.u2()? },
            15 => Constant::MethodHandle { kind: r.u1()?, reference: r.u2()? },
            16 => Constant::MethodType { descriptor: r.u2()? },
            17 => Constant::Dynamic { bootstrap: r.u2()?, name_and_type: r.u2()? },
            18 => {
                Constant::InvokeDynamic { bootstrap: r.u2()?, name_and_type: r.u2()? }
            }
            19 => Constant::Module { name: r.u2()? },
            20 => Constant::Package { name: r.u2()? },
            tag => {
                let m = format!("unknown constant pool tag {tag} at offset {at}");
                return Err(Error::ClassFormat(m));
            }
        })
    }
    // @end
}

// @listing text
/// A decoded `Utf8` entry: a `String` if the code units are valid UTF-16.
fn text(units: Vec<u16>) -> Constant {
    match String::from_utf16(&units) {
        Ok(s) => Constant::Utf8(s),
        Err(_) => Constant::Utf16(units),
    }
}

/// The eight bytes of a `Long` or `Double`, which the JVMS calls `high_bytes`
/// and `low_bytes`.
fn eight(r: &mut Reader) -> Result<u64> {
    let high = r.u4()? as u64;
    let low = r.u4()? as u64;
    Ok(high << 32 | low)
}
// @end

// @listing pool
/// A class file's constant pool. `entries[i]` is entry `i`, so entry 0 and
/// the entry after each `Long` and `Double` are `Constant::Unusable`.
#[derive(Debug)]
pub struct ConstantPool {
    entries: Vec<Constant>,
}
// @end

// @listing member-ref
/// What a `Fieldref`, `Methodref` or `InterfaceMethodref` names.
#[derive(Debug, PartialEq)]
pub struct MemberRef<'a> {
    pub class: &'a str,
    pub name: &'a str,
    pub descriptor: &'a str,
}
// @end

impl ConstantPool {
    // @listing pool-read
    /// Reads `constant_pool_count` and then the entries, which number one
    /// fewer than the count says, with longs and doubles counting twice.
    pub fn read(r: &mut Reader) -> Result<ConstantPool> {
        let count = r.u2()? as usize;
        let mut entries = Vec::with_capacity(count);
        entries.push(Constant::Unusable);
        while entries.len() < count {
            let c = Constant::read(r)?;
            let wide = matches!(c, Constant::Long(_) | Constant::Double(_));
            entries.push(c);
            if wide {
                entries.push(Constant::Unusable);
            }
        }
        if entries.len() > count {
            let m = format!("the entries overrun constant_pool_count {count}");
            return Err(Error::ClassFormat(m));
        }
        Ok(ConstantPool { entries })
    }
    // @end

    // @listing pool-get
    /// `constant_pool_count`: one more than the highest index.
    pub fn count(&self) -> u16 {
        self.entries.len() as u16
    }

    /// The entry at `index`, if that is a usable index.
    pub fn get(&self, index: u16) -> Result<&Constant> {
        match self.entries.get(index as usize) {
            None | Some(Constant::Unusable) => Err(Error::ClassFormat(format!(
                "#{index} is not a usable constant pool index"
            ))),
            Some(c) => Ok(c),
        }
    }

    // @end

    // @listing pool-iter
    /// Every usable entry with its index.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &Constant)> {
        let usable = |(_, c): &(usize, &Constant)| *c != &Constant::Unusable;
        self.entries.iter().enumerate().filter(usable).map(|(i, c)| (i as u16, c))
    }
    // @end

    // @listing pool-typed
    /// The text of the `Utf8` entry at `index`.
    pub fn utf8(&self, index: u16) -> Result<&str> {
        match self.get(index)? {
            Constant::Utf8(s) => Ok(s),
            other => Err(wrong(index, other, "Utf8")),
        }
    }

    /// The name of the `Class` entry at `index`, such as `java/lang/Object`.
    pub fn class_name(&self, index: u16) -> Result<&str> {
        match self.get(index)? {
            Constant::Class { name } => self.utf8(*name),
            other => Err(wrong(index, other, "Class")),
        }
    }

    /// The name and descriptor of the `NameAndType` entry at `index`.
    pub fn name_and_type(&self, index: u16) -> Result<(&str, &str)> {
        match self.get(index)? {
            Constant::NameAndType { name, descriptor } => {
                Ok((self.utf8(*name)?, self.utf8(*descriptor)?))
            }
            other => Err(wrong(index, other, "NameAndType")),
        }
    }
    // @end

    // @listing pool-member
    /// The class, name and descriptor of the field or method that the
    /// `Fieldref`, `Methodref` or `InterfaceMethodref` at `index` names.
    pub fn member(&self, index: u16) -> Result<MemberRef<'_>> {
        let (class, nat) = match *self.get(index)? {
            Constant::Fieldref { class, name_and_type }
            | Constant::Methodref { class, name_and_type }
            | Constant::InterfaceMethodref { class, name_and_type } => {
                (class, name_and_type)
            }
            ref other => {
                return Err(wrong(index, other, "field or method reference"));
            }
        };
        let (name, descriptor) = self.name_and_type(nat)?;
        Ok(MemberRef { class: self.class_name(class)?, name, descriptor })
    }
    // @end

    // @listing comment
    /// What the entry at `index` leads to, as `javap -v` shows it after `//`:
    /// for a `Methodref`, the class, name and descriptor it names.
    pub(crate) fn comment(&self, index: u16, c: &Constant) -> Result<String> {
        Ok(match *c {
            Constant::String { utf8 } => match self.get(utf8)? {
                c @ (Constant::Utf8(_) | Constant::Utf16(_)) => c.describe().1,
                other => return Err(wrong(utf8, other, "Utf8")),
            },
            Constant::Class { name } => quote_class(self.utf8(name)?),
            Constant::MethodType { descriptor: name }
            | Constant::Module { name }
            | Constant::Package { name } => self.utf8(name)?.to_string(),
            Constant::Fieldref { .. }
            | Constant::Methodref { .. }
            | Constant::InterfaceMethodref { .. } => {
                let m = self.member(index)?;
                format!("{}.{}", quote_class(m.class), quote(m.name, m.descriptor))
            }
            Constant::NameAndType { .. } => {
                let (name, descriptor) = self.name_and_type(index)?;
                quote(name, descriptor)
            }
            Constant::Dynamic { bootstrap, name_and_type }
            | Constant::InvokeDynamic { bootstrap, name_and_type } => {
                let (name, descriptor) = self.name_and_type(name_and_type)?;
                format!("#{bootstrap}:{}", quote(name, descriptor))
            }
            _ => String::new(),
        })
    }
    // @end
}

// @listing quote
/// `name:descriptor`, with `javap`'s quotes around `<init>` and `<clinit>`.
fn quote(name: &str, descriptor: &str) -> String {
    match name.starts_with('<') {
        true => format!("\"{name}\":{descriptor}"),
        false => format!("{name}:{descriptor}"),
    }
}

/// A class name, in quotes if it is an array class's: `"[I"`.
fn quote_class(name: &str) -> String {
    match name.starts_with('[') {
        true => format!("\"{name}\""),
        false => name.to_string(),
    }
}

/// UTF-16 as text, with control characters and unpaired surrogates written
/// as `javap` writes them, `\u0000`.
fn escape(units: &[u16]) -> String {
    let unit = |u: u32| format!("\\u{u:04x}");
    char::decode_utf16(units.iter().copied())
        .map(|c| match c {
            Ok(c) if c.is_control() => unit(c as u32),
            Ok(c) => c.to_string(),
            Err(e) => unit(e.unpaired_surrogate() as u32),
        })
        .collect()
}
// @end

// @listing wrong
/// The error for an index that leads to the wrong kind of entry.
fn wrong(index: u16, found: &Constant, wanted: &str) -> Error {
    Error::ClassFormat(format!("#{index} is {found:?}, not a {wanted}"))
}
// @end

// @listing display
impl fmt::Display for ConstantPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, c) in self.iter() {
            let (tag, operands) = c.describe();
            let shown = self.comment(index, c).unwrap_or_else(|e| e.to_string());
            let index = format!("#{index}");
            match shown.is_empty() {
                true => writeln!(f, "{index:>4} = {tag:<11} {operands}")?,
                false => {
                    writeln!(f, "{index:>4} = {tag:<11} {operands:<7} // {shown}")?
                }
            }
        }
        Ok(())
    }
}
// @end
