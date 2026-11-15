//! Chapter 17: Call sites and string concatenation.

use crate::class_file::ClassFile;
use crate::dynamic::bootstrap_methods;
use crate::error::Result;
use crate::pool::{Constant, ConstantPool};
use std::collections::BTreeMap;
use std::path::Path;

// @listing bootstrap-listing
/// The `BootstrapMethods` section of `javap -v`: each bootstrap method as a
/// method handle, then its static arguments.
pub fn bootstrap_listing(file: &ClassFile) -> Result<String> {
    let pool = &file.constant_pool;
    let mut out = String::from("BootstrapMethods:\n");
    for (n, b) in bootstrap_methods(file)?.iter().enumerate() {
        out += &format!("  {n}: #{} {}\n", b.method, argument(pool, b.method)?);
        out += "    Method arguments:\n";
        for &arg in &b.args {
            out += &format!("      #{arg} {}\n", argument(pool, arg)?);
        }
    }
    Ok(out)
}

/// A constant as `javap` prints it as a bootstrap argument.
fn argument(pool: &ConstantPool, index: u16) -> Result<String> {
    const KINDS: [&str; 9] = [
        "getField",
        "getStatic",
        "putField",
        "putStatic",
        "invokeVirtual",
        "invokeStatic",
        "invokeSpecial",
        "newInvokeSpecial",
        "invokeInterface",
    ];
    Ok(match *pool.get(index)? {
        Constant::MethodHandle { kind, reference } => {
            let kind = KINDS.get(usize::from(kind).wrapping_sub(1)).unwrap_or(&"?");
            format!("REF_{kind} {}", pool.comment(reference, pool.get(reference)?)?)
        }
        ref c @ (Constant::Integer(_)
        | Constant::Long(_)
        | Constant::Float(_)
        | Constant::Double(_)) => c.describe().1,
        ref c => pool.comment(index, c)?,
    })
}
// @end

// @listing bootstrap-census
/// How many `InvokeDynamic` constants in the class files under `dir` name
/// each bootstrap method, by its class and name.
pub fn bootstrap_census(dir: &Path) -> Result<BTreeMap<String, usize>> {
    let mut counts = BTreeMap::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "class") {
                let file = ClassFile::parse(&std::fs::read(&path)?)?;
                let bootstraps = bootstrap_methods(&file)?;
                let pool = &file.constant_pool;
                for (_, c) in pool.iter() {
                    let &Constant::InvokeDynamic { bootstrap, .. } = c else {
                        continue;
                    };
                    let Some(b) = bootstraps.get(usize::from(bootstrap)) else {
                        continue;
                    };
                    let &Constant::MethodHandle { reference, .. } =
                        pool.get(b.method)?
                    else {
                        continue;
                    };
                    let m = pool.member(reference)?;
                    *counts.entry(format!("{}.{}", m.class, m.name)).or_insert(0) +=
                        1;
                }
            }
        }
    }
    Ok(counts)
}
// @end
