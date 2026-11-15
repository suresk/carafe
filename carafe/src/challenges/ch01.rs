//! Chapter 1: a class file is a stream of bytes.

use crate::error::{Error, Result};
use crate::pool::{Constant, ConstantPool};
use std::collections::BTreeMap;

// @listing encode
/// Encodes UTF-16 code units as modified UTF-8: the inverse of
/// `decode_modified_utf8`.
pub fn encode_modified_utf8(units: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(units.len());
    for &u in units {
        match u {
            0x0001..=0x007f => out.push(u as u8),
            0x0000 | 0x0080..=0x07ff => {
                out.extend([0xc0 | (u >> 6) as u8, 0x80 | (u & 0x3f) as u8]);
            }
            _ => out.extend([
                0xe0 | (u >> 12) as u8,
                0x80 | (u >> 6 & 0x3f) as u8,
                0x80 | (u & 0x3f) as u8,
            ]),
        }
    }
    out
}
// @end

// @listing check-pool
/// Checks that every index in the pool leads to the kind of entry JVMS 4.4
/// requires, and that each method handle's kind is one of the nine.
pub fn check_pool(pool: &ConstantPool) -> Result<()> {
    for (index, c) in pool.iter() {
        let bad = |m: String| Err(Error::ClassFormat(format!("#{index}: {m}")));
        match *c {
            Constant::Class { name }
            | Constant::MethodType { descriptor: name }
            | Constant::Module { name }
            | Constant::Package { name } => {
                pool.utf8(name)?;
            }
            Constant::String { utf8 } => match pool.get(utf8)? {
                Constant::Utf8(_) | Constant::Utf16(_) => {}
                other => return bad(format!("#{utf8} is {other:?}, not a Utf8")),
            },
            Constant::Fieldref { .. }
            | Constant::Methodref { .. }
            | Constant::InterfaceMethodref { .. } => {
                pool.member(index)?;
            }
            Constant::NameAndType { .. } => {
                pool.name_and_type(index)?;
            }
            Constant::Dynamic { name_and_type, .. }
            | Constant::InvokeDynamic { name_and_type, .. } => {
                pool.name_and_type(name_and_type)?;
            }
            Constant::MethodHandle { kind, reference } => {
                let target = pool.get(reference)?;
                if !handle_target(kind, target) {
                    return bad(format!("kind {kind} cannot refer to {target:?}"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
// @end

// @listing handle-target
/// Whether a method handle of `kind` may refer to `target` (JVMS 4.4.8).
fn handle_target(kind: u8, target: &Constant) -> bool {
    use Constant::{Fieldref, InterfaceMethodref, Methodref};
    matches!(
        (kind, target),
        (1..=4, Fieldref { .. })
            | (5 | 8, Methodref { .. })
            | (6 | 7, Methodref { .. } | InterfaceMethodref { .. })
            | (9, InterfaceMethodref { .. })
    )
}
// @end

// @listing tag-counts
/// How many entries of each tag the pool holds, by the tag's name.
pub fn tag_counts(pool: &ConstantPool) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for (_, c) in pool.iter() {
        *counts.entry(c.describe().0).or_insert(0) += 1;
    }
    counts
}
// @end

// @listing write-pool
/// Writes the pool back out as a class file holds it, count first.
pub fn write_pool(pool: &ConstantPool) -> Vec<u8> {
    let mut out = pool.count().to_be_bytes().to_vec();
    for (_, c) in pool.iter() {
        match *c {
            Constant::Unusable => {}
            Constant::Utf8(ref s) => {
                utf8(&mut out, &s.encode_utf16().collect::<Vec<_>>())
            }
            Constant::Utf16(ref units) => utf8(&mut out, units),
            Constant::Integer(v) => tagged(&mut out, 3, &v.to_be_bytes()),
            Constant::Float(v) => tagged(&mut out, 4, &v.to_bits().to_be_bytes()),
            Constant::Long(v) => tagged(&mut out, 5, &v.to_be_bytes()),
            Constant::Double(v) => tagged(&mut out, 6, &v.to_bits().to_be_bytes()),
            Constant::Class { name } => u2s(&mut out, 7, &[name]),
            Constant::String { utf8 } => u2s(&mut out, 8, &[utf8]),
            Constant::Fieldref { class, name_and_type } => {
                u2s(&mut out, 9, &[class, name_and_type])
            }
            Constant::Methodref { class, name_and_type } => {
                u2s(&mut out, 10, &[class, name_and_type])
            }
            Constant::InterfaceMethodref { class, name_and_type } => {
                u2s(&mut out, 11, &[class, name_and_type])
            }
            Constant::NameAndType { name, descriptor } => {
                u2s(&mut out, 12, &[name, descriptor])
            }
            Constant::MethodHandle { kind, reference } => {
                tagged(&mut out, 15, &[kind]);
                out.extend(reference.to_be_bytes());
            }
            Constant::MethodType { descriptor } => u2s(&mut out, 16, &[descriptor]),
            Constant::Dynamic { bootstrap, name_and_type } => {
                u2s(&mut out, 17, &[bootstrap, name_and_type])
            }
            Constant::InvokeDynamic { bootstrap, name_and_type } => {
                u2s(&mut out, 18, &[bootstrap, name_and_type])
            }
            Constant::Module { name } => u2s(&mut out, 19, &[name]),
            Constant::Package { name } => u2s(&mut out, 20, &[name]),
        }
    }
    out
}

fn u2s(out: &mut Vec<u8>, tag: u8, values: &[u16]) {
    out.push(tag);
    values.iter().for_each(|v| out.extend(v.to_be_bytes()));
}

fn tagged(out: &mut Vec<u8>, tag: u8, bytes: &[u8]) {
    out.push(tag);
    out.extend(bytes);
}

fn utf8(out: &mut Vec<u8>, units: &[u16]) {
    let bytes = encode_modified_utf8(units);
    out.push(1);
    out.extend((bytes.len() as u16).to_be_bytes());
    out.extend(bytes);
}
// @end
