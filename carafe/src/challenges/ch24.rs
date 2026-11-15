//! Chapter 24: The scoreboard.

use crate::interpreter::Vm;
use std::collections::{BTreeMap, BTreeSet};

// @listing startup-classes
/// The classes a program loaded before its own main class: what carafe's
/// startup and `run_main` cost every program.
pub fn startup_classes<'a>(vm: &'a Vm, main: &str) -> &'a [String] {
    let end = vm.loaded.iter().position(|c| c == main).unwrap_or(vm.loaded.len());
    &vm.loaded[..end]
}
// @end

// @listing natives-by-class
/// How many of a program's natives each class declares.
pub fn natives_by_class(natives: &BTreeSet<String>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for native in natives {
        let class = native.split('.').next().unwrap_or(native);
        *counts.entry(class.to_string()).or_insert(0) += 1;
    }
    counts
}
// @end
