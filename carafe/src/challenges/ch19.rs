//! Chapter 19: Records, enums and pattern `switch`.

use crate::class_file::ClassFile;
use crate::error::Result;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::reader::Reader;
use crate::value::Value;

// @listing record-components
/// The components a record class declares in its `Record` attribute (JVMS
/// 4.7.30), as names and descriptors, in order; empty for any other class.
pub fn record_components(file: &ClassFile) -> Result<Vec<(String, String)>> {
    let Some(record) = file.attributes.iter().find(|a| a.name == "Record") else {
        return Ok(Vec::new());
    };
    let pool = &file.constant_pool;
    let mut r = Reader::new(&record.info);
    let mut components = Vec::new();
    for _ in 0..r.u2()? {
        let (name, descriptor) = (r.u2()?, r.u2()?);
        for _ in 0..r.u2()? {
            let (_, length) = (r.u2()?, r.u4()?);
            r.take(length as usize)?; // the component's own attributes
        }
        components
            .push((pool.utf8(name)?.to_string(), pool.utf8(descriptor)?.to_string()));
    }
    Ok(components)
}
// @end

// @listing enum-switch
/// `SwitchBootstraps.enumSwitch` with labels that are enum constants, by
/// name: the index of the first label from `restart` that names `target`,
/// -1 for `null`, or the number of labels if none does.
pub fn enum_switch(
    vm: &mut Vm,
    names: &[&str],
    target: Ref,
    restart: i32,
) -> Result<i32> {
    if target == Ref::NULL {
        return Ok(-1);
    }
    let enum_class = vm.load("java/lang/Enum")?;
    let slot = enum_class.field("name", "Ljava/lang/String;")?;
    let Value::Ref(name) = vm.heap.read(target.0 + slot.offset, &slot.ty) else {
        unreachable!("a reference field")
    };
    let name = vm.rust_string(name)?;
    let start = usize::try_from(restart).unwrap_or(0);
    let found = names.iter().skip(start).position(|&n| n == name);
    Ok(found.map_or(names.len() as i32, |i| (start + i) as i32))
}
// @end
