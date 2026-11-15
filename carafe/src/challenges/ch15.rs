//! Chapter 15: Class objects, hash codes and HashMap.

use crate::error::Result;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::value::Value;
use std::collections::BTreeMap;

// @listing get-interfaces
/// `Class.getInterfaces0()`: the mirrors of the class's direct
/// superinterfaces, in declaration order, as a `Class[]`.
pub fn get_interfaces(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let class = vm.class_of_mirror(mirror)?;
    let class_type = crate::descriptor::FieldType::Object("java/lang/Class".into());
    let array = vm.heap.new_array(&class_type, class.interfaces.len() as u32)?;
    vm.heap.roots.push(array);
    for (i, interface) in class.interfaces.iter().enumerate() {
        let interface = vm.mirror(interface)?;
        let array = *vm.heap.roots.last().expect("pushed above");
        let at = vm.heap.element(array, i as i32)?;
        vm.heap.write(at, &class_type, Value::Ref(interface))?;
    }
    Ok(Some(Value::Ref(vm.heap.roots.pop().expect("pushed above"))))
}
// @end

// @listing census
/// How many objects and arrays of each class the heap holds, by name.
pub fn census(vm: &mut Vm) -> Result<BTreeMap<String, usize>> {
    let mut counts = BTreeMap::new();
    for r in crate::challenges::ch12::objects_and_arrays(&vm.heap) {
        let class = vm.class_of(Ref(r.0))?;
        *counts.entry(class.file.this_class.clone()).or_insert(0) += 1;
    }
    Ok(counts)
}
// @end
