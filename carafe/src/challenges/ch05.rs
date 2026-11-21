//! Chapter 5: calls and returns.

use crate::bytecode::Operand;
use crate::class_file::ClassFile;
use crate::error::Result;
use crate::opcode::INVOKESTATIC;
use std::collections::{BTreeMap, BTreeSet};

// @listing call-graph
/// For each method of `class` (as `name` + descriptor), the methods its
/// `invokestatic` instructions call (as `class.name` + descriptor).
pub fn call_graph(class: &ClassFile) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut graph = BTreeMap::new();
    for method in &class.methods {
        let mut callees = BTreeSet::new();
        if let Some(code) = &method.code {
            for i in code.decode()? {
                if let (INVOKESTATIC, Operand::Constant(index)) =
                    (i.opcode, i.operand)
                {
                    let m = class.constant_pool.member(index)?;
                    callees.insert(format!("{}.{}{}", m.class, m.name, m.descriptor));
                }
            }
        }
        graph.insert(format!("{}{}", method.name, method.descriptor), callees);
    }
    Ok(graph)
}

/// The methods of `class` that can call themselves, directly or through
/// other methods of the class.
pub fn recursive(class: &ClassFile) -> Result<BTreeSet<String>> {
    let graph = call_graph(class)?;
    let own = format!("{}.", class.this_class);
    let mut found = BTreeSet::new();
    for (start, callees) in &graph {
        let mut seen = BTreeSet::new();
        let mut work: Vec<&String> = callees.iter().collect();
        while let Some(callee) = work.pop() {
            let Some(name) = callee.strip_prefix(&own) else { continue };
            if name == start {
                found.insert(start.clone());
                break;
            }
            if seen.insert(name.to_string()) {
                work.extend(graph.get(name).into_iter().flatten());
            }
        }
    }
    Ok(found)
}
// @end
