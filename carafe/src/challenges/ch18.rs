//! Chapter 18: Lambdas.

use crate::class::Class;
use crate::class_file::{ClassFile, MAGIC};
use crate::dynamic::CallSite;
use crate::error::Result;
use crate::pool::Constant;

// @listing captures
/// For each lambda call site in `class` that has been linked, in the order
/// of the methods and then the pcs, the name of its lambda class and the
/// descriptors of the values it captures.
pub fn captures(class: &Class) -> Vec<(String, Vec<String>)> {
    let sites = class.call_sites.borrow();
    let mut keys: Vec<&(usize, u16)> = sites.keys().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| match &*sites[key] {
            CallSite::Lambda { class, captures, .. } => {
                let captures = captures.iter().map(|t| t.descriptor()).collect();
                Some((class.file.this_class.clone(), captures))
            }
            _ => None,
        })
        .collect()
}
// @end

// @listing write-class
/// The bytes of a class file for `file`, such as one of carafe's lambda
/// classes. Names that the pool lacks, such as `Code` and the names and
/// descriptors of the members, are added to the end of it.
pub fn write_class(file: &ClassFile) -> Result<Vec<u8>> {
    let pool = &file.constant_pool;
    let mut w = Writer {
        pool: (0..pool.count())
            .map(|i| pool.get(i).cloned().unwrap_or(Constant::Unusable))
            .collect(),
        out: Vec::new(),
    };
    w.u2(file.access_flags.0);
    let this = w.class(&file.this_class);
    w.u2(this);
    let superclass = file.super_class.as_ref().map_or(0, |s| w.class(s));
    w.u2(superclass);
    w.u2(file.interfaces.len() as u16);
    for interface in &file.interfaces {
        let index = w.class(interface);
        w.u2(index);
    }
    w.u2(file.fields.len() as u16);
    for f in &file.fields {
        w.member(f.access_flags.0, &f.name, &f.descriptor);
        w.attributes(
            &f.attributes
                .iter()
                .map(|a| (a.name.clone(), a.info.clone()))
                .collect::<Vec<_>>(),
        );
    }
    w.u2(file.methods.len() as u16);
    for m in &file.methods {
        w.member(m.access_flags.0, &m.name, &m.descriptor);
        let mut attributes: Vec<(String, Vec<u8>)> = Vec::new();
        if let Some(code) = &m.code {
            let mut info = Vec::new();
            info.extend(code.max_stack.to_be_bytes());
            info.extend(code.max_locals.to_be_bytes());
            info.extend((code.bytecode.len() as u32).to_be_bytes());
            info.extend(&code.bytecode);
            info.extend((code.handlers.len() as u16).to_be_bytes());
            for h in &code.handlers {
                for v in [h.start_pc, h.end_pc, h.handler_pc, h.catch_type] {
                    info.extend(v.to_be_bytes());
                }
            }
            info.extend(0u16.to_be_bytes()); // no attributes of its own
            attributes.push(("Code".to_string(), info));
        }
        attributes.extend(
            m.attributes
                .iter()
                .filter(|a| a.name != "Code")
                .map(|a| (a.name.clone(), a.info.clone())),
        );
        w.attributes(&attributes);
    }
    w.attributes(
        &file
            .attributes
            .iter()
            .map(|a| (a.name.clone(), a.info.clone()))
            .collect::<Vec<_>>(),
    );
    let mut bytes = MAGIC.to_be_bytes().to_vec();
    bytes.extend(file.minor_version.to_be_bytes());
    bytes.extend(file.major_version.to_be_bytes());
    bytes.extend((w.pool.len() as u16).to_be_bytes());
    for c in &w.pool {
        bytes.extend(encode(c));
    }
    bytes.extend(w.out);
    Ok(bytes)
}

/// The pool, which grows as names are added, and the bytes after it.
struct Writer {
    pool: Vec<Constant>,
    out: Vec<u8>,
}

impl Writer {
    fn u2(&mut self, v: u16) {
        self.out.extend(v.to_be_bytes());
    }

    fn find_or_add(&mut self, c: Constant) -> u16 {
        match self.pool.iter().position(|e| *e == c) {
            Some(i) => i as u16,
            None => {
                self.pool.push(c);
                self.pool.len() as u16 - 1
            }
        }
    }

    fn utf8(&mut self, s: &str) -> u16 {
        self.find_or_add(Constant::Utf8(s.to_string()))
    }

    fn class(&mut self, name: &str) -> u16 {
        let name = self.utf8(name);
        self.find_or_add(Constant::Class { name })
    }

    fn member(&mut self, flags: u16, name: &str, descriptor: &str) {
        let (name, descriptor) = (self.utf8(name), self.utf8(descriptor));
        for v in [flags, name, descriptor] {
            self.u2(v);
        }
    }

    fn attributes(&mut self, attributes: &[(String, Vec<u8>)]) {
        self.u2(attributes.len() as u16);
        for (name, info) in attributes {
            let name = self.utf8(name);
            self.u2(name);
            self.out.extend((info.len() as u32).to_be_bytes());
            self.out.extend(info);
        }
    }
}

/// A constant as the class file stores it: its tag, then its contents.
/// `Unusable`, the slot after a `long` or `double`, takes no bytes.
fn encode(c: &Constant) -> Vec<u8> {
    let pair = |tag: u8, a: u16, b: u16| {
        [vec![tag], a.to_be_bytes().into(), b.to_be_bytes().into()].concat()
    };
    match *c {
        Constant::Unusable => Vec::new(),
        Constant::Utf8(ref s) => utf8_entry(&s.encode_utf16().collect::<Vec<_>>()),
        Constant::Utf16(ref units) => utf8_entry(units),
        Constant::Integer(v) => [vec![3], v.to_be_bytes().into()].concat(),
        Constant::Float(v) => [vec![4], v.to_bits().to_be_bytes().into()].concat(),
        Constant::Long(v) => [vec![5], v.to_be_bytes().into()].concat(),
        Constant::Double(v) => [vec![6], v.to_bits().to_be_bytes().into()].concat(),
        Constant::Class { name } => [vec![7], name.to_be_bytes().into()].concat(),
        Constant::String { utf8 } => [vec![8], utf8.to_be_bytes().into()].concat(),
        Constant::Fieldref { class, name_and_type } => pair(9, class, name_and_type),
        Constant::Methodref { class, name_and_type } => {
            pair(10, class, name_and_type)
        }
        Constant::InterfaceMethodref { class, name_and_type } => {
            pair(11, class, name_and_type)
        }
        Constant::NameAndType { name, descriptor } => pair(12, name, descriptor),
        Constant::MethodHandle { kind, reference } => {
            [vec![15, kind], reference.to_be_bytes().into()].concat()
        }
        Constant::MethodType { descriptor } => {
            [vec![16], descriptor.to_be_bytes().into()].concat()
        }
        Constant::Dynamic { bootstrap, name_and_type } => {
            pair(17, bootstrap, name_and_type)
        }
        Constant::InvokeDynamic { bootstrap, name_and_type } => {
            pair(18, bootstrap, name_and_type)
        }
        Constant::Module { name } => [vec![19], name.to_be_bytes().into()].concat(),
        Constant::Package { name } => [vec![20], name.to_be_bytes().into()].concat(),
    }
}

/// A `Utf8` entry: tag 1, a length, and the text in modified UTF-8, which
/// writes U+0000 in two bytes and each half of a surrogate pair in three.
fn utf8_entry(units: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for &u in units {
        match u {
            0x01..=0x7f => bytes.push(u as u8),
            0x00 | 0x80..=0x7ff => {
                bytes.extend([0xc0 | (u >> 6) as u8, 0x80 | (u & 0x3f) as u8])
            }
            _ => bytes.extend([
                0xe0 | (u >> 12) as u8,
                0x80 | ((u >> 6) & 0x3f) as u8,
                0x80 | (u & 0x3f) as u8,
            ]),
        }
    }
    [vec![1], (bytes.len() as u16).to_be_bytes().into(), bytes].concat()
}
// @end
