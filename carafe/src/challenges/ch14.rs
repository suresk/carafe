//! Chapter 14: natives, Unsafe and the shape of an int.

use crate::error::Result;
use crate::flags::AccessFlags;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::natives::implemented;
use crate::value::Value;

// @listing identity-hash
/// `System.identityHashCode(Object)`: the hash in the object's header, which
/// `Object.hashCode` also returns, and 0 for `null`.
pub fn identity_hash_code(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(object)] = *args else { unreachable!("checked descriptor") };
    let hash = match object {
        Ref::NULL => 0,
        object => vm.heap.hash(object) as i32,
    };
    Ok(Some(Value::Int(hash)))
}
// @end

// @listing coverage
/// How many native methods the class `name` declares, and how many of them
/// carafe implements.
pub fn native_coverage(vm: &mut Vm, name: &str) -> Result<(usize, usize)> {
    let class = vm.load(name)?;
    let natives: Vec<_> = class
        .file
        .methods
        .iter()
        .filter(|m| m.access_flags.has(AccessFlags::NATIVE))
        .collect();
    let done =
        natives.iter().filter(|m| implemented(name, &m.name, &m.descriptor)).count();
    Ok((natives.len(), done))
}
// @end
