//! Chapter 21: Streams.

use crate::interpreter::Vm;
use std::collections::BTreeMap;

// @listing packages
/// How many of the classes a program loaded are in each package.
pub fn packages(vm: &Vm) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for name in &vm.loaded {
        let package = name.rsplit_once('/').map_or("", |(package, _)| package);
        *counts.entry(package.to_string()).or_insert(0) += 1;
    }
    counts
}
// @end

// @listing lambda-callers
/// How many lambda classes carafe made for each class that contains lambda
/// call sites, by the class's name.
pub fn lambda_callers(vm: &Vm) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for name in vm.classes.keys() {
        if let Some((caller, _)) = name.split_once("$$Lambda$") {
            *counts.entry(caller.to_string()).or_insert(0) += 1;
        }
    }
    counts
}
// @end
