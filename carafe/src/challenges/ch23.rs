//! Chapter 23: How fast is carafe?

use crate::cache::Resolved;
use crate::class::Class;
use std::collections::BTreeMap;

// @listing histogram
/// How many times each instruction appears in a class's methods, by name.
pub fn histogram(class: &Class) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for m in 0..class.file.methods.len() {
        let Ok(code) = class.instructions(m) else { continue }; // abstract or native
        for instruction in code.iter() {
            *counts.entry(instruction.mnemonic()).or_insert(0) += 1;
        }
    }
    counts
}
// @end

// @listing cache-census
/// How many of a class's constant pool entries the cache holds, of each kind.
pub fn cache_census(class: &Class) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for index in 1..class.file.constant_pool.count() {
        let kind = match class.cached(index) {
            Some(Resolved::Class(_)) => "class",
            Some(Resolved::Method(..)) => "method",
            Some(Resolved::Field(_)) => "field",
            Some(Resolved::Static(..)) => "static",
            None => continue,
        };
        *counts.entry(kind).or_insert(0) += 1;
    }
    counts
}
// @end
