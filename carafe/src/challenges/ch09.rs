//! Chapter 9: initialization.

use crate::bytecode::Operand;
use crate::class::Class;
use crate::class_file::ClassFile;
use crate::error::Result;
use crate::flags::AccessFlags;
use crate::opcode::{GETSTATIC, INVOKESTATIC, NEW, PUTSTATIC};
use crate::value::Value;
use std::collections::{BTreeMap, BTreeSet};

// @listing static-values
/// The class's static fields and their current values, by name.
pub fn static_values(class: &Class) -> Vec<(String, Value)> {
    let fields = class.file.fields.iter();
    let statics = fields.filter(|f| f.access_flags.has(AccessFlags::STATIC));
    let values = class.statics.borrow();
    statics.zip(values.iter()).map(|(f, v)| (f.name.clone(), *v)).collect()
}
// @end

// @listing init-cycles
/// The pairs of classes whose static initializers can each trigger the
/// other's initialization, directly: `(a, b)` with `a < b`.
pub fn init_cycles(files: &[ClassFile]) -> Result<BTreeSet<(String, String)>> {
    let mut triggers: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let mut named = BTreeSet::new();
        let clinit = file.methods.iter().find(|m| m.name == "<clinit>");
        if let Some(code) = clinit.and_then(|m| m.code.as_ref()) {
            for i in code.decode()? {
                let Operand::Constant(index) = i.operand else { continue };
                let class = match i.opcode {
                    NEW => file.constant_pool.class_name(index)?,
                    GETSTATIC | PUTSTATIC | INVOKESTATIC => {
                        file.constant_pool.member(index)?.class
                    }
                    _ => continue,
                };
                if class != file.this_class {
                    named.insert(class.to_string());
                }
            }
        }
        triggers.insert(&file.this_class, named);
    }
    let mut cycles = BTreeSet::new();
    for (&a, named) in &triggers {
        for b in named {
            let back = triggers.get(b.as_str()).is_some_and(|n| n.contains(a));
            if back && a < b.as_str() {
                cycles.insert((a.to_string(), b.clone()));
            }
        }
    }
    Ok(cycles)
}
// @end
