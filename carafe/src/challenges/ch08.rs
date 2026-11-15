//! Chapter 8: loading classes.

use crate::class_file::ClassFile;
use crate::descriptor::{FieldType, MethodDescriptor};
use crate::error::Result;
use crate::pool::Constant;
use crate::reader::Reader;
use std::collections::BTreeSet;

// @listing references
/// Every class a class file names: in `Class` constants, and in the
/// descriptors of its members and of the members it refers to.
pub fn references(file: &ClassFile) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    let mut add = |ty: &FieldType| {
        let mut ty = ty;
        while let FieldType::Array(element) = ty {
            ty = element;
        }
        if let FieldType::Object(name) = ty {
            names.insert(name.clone());
        }
    };
    let mut descriptors: Vec<String> = Vec::new();
    for (_, constant) in file.constant_pool.iter() {
        match constant {
            Constant::Class { name } => {
                let name = file.constant_pool.utf8(*name)?;
                match name.starts_with('[') {
                    true => descriptors.push(name.to_string()),
                    false => descriptors.push(format!("L{name};")),
                }
            }
            Constant::NameAndType { descriptor, .. } => {
                descriptors.push(file.constant_pool.utf8(*descriptor)?.to_string());
            }
            _ => {}
        }
    }
    descriptors.extend(file.fields.iter().map(|f| f.descriptor.clone()));
    descriptors.extend(file.methods.iter().map(|m| m.descriptor.clone()));
    for d in descriptors {
        if d.starts_with('(') {
            let ty = MethodDescriptor::parse(&d)?;
            ty.params.iter().chain(&ty.ret).for_each(&mut add);
        } else {
            add(&FieldType::parse(&d)?);
        }
    }
    names.remove(&file.this_class);
    Ok(names)
}
// @end

// @listing nestmates
/// JVMS 5.4.4's full nestmate test: `member` names `host` in its `NestHost`
/// attribute, and `host` lists `member` in its `NestMembers` attribute.
pub fn nestmates(host: &ClassFile, member: &ClassFile) -> Result<bool> {
    let class_names = |file: &ClassFile, attribute: &str| -> Result<Vec<String>> {
        let Some(a) = file.attributes.iter().find(|a| a.name == attribute) else {
            return Ok(Vec::new());
        };
        let mut r = Reader::new(&a.info);
        let count = if attribute == "NestHost" { 1 } else { r.u2()? };
        let mut names = Vec::new();
        for _ in 0..count {
            names.push(file.constant_pool.class_name(r.u2()?)?.to_string());
        }
        Ok(names)
    };
    let names_host = class_names(member, "NestHost")? == [host.this_class.clone()];
    let lists_member = class_names(host, "NestMembers")?.contains(&member.this_class);
    Ok(names_host && lists_member)
}
// @end
