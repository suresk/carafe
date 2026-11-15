//! Chapter 22: A main thread and standard input.

use crate::error::Result;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::value::Value;

// @listing thread-summary
/// The main thread's name, its group's name and its priority, read from
/// the heap, or `None` if the program never asked for a thread.
pub fn thread_summary(vm: &mut Vm) -> Result<Option<(String, String, i32)>> {
    let Some(thread) = vm.thread else { return Ok(None) };
    let field = |vm: &mut Vm,
                 class: &str,
                 object: Ref,
                 name: &str,
                 ty: &str|
     -> Result<Value> {
        let class = vm.load(class)?;
        let slot = class.field(name, ty)?;
        Ok(vm.heap.read(object.0 + slot.offset, &slot.ty))
    };
    let thread_class = "java/lang/Thread";
    let Value::Ref(name) =
        field(vm, thread_class, thread, "name", "Ljava/lang/String;")?
    else {
        unreachable!("a String field")
    };
    let holder_type = "Ljava/lang/Thread$FieldHolder;";
    let Value::Ref(holder) = field(vm, thread_class, thread, "holder", holder_type)?
    else {
        unreachable!("a FieldHolder field")
    };
    let holders = "java/lang/Thread$FieldHolder";
    let Value::Ref(group) =
        field(vm, holders, holder, "group", "Ljava/lang/ThreadGroup;")?
    else {
        unreachable!("a ThreadGroup field")
    };
    let Value::Int(priority) = field(vm, holders, holder, "priority", "I")? else {
        unreachable!("an int field")
    };
    let groups = "java/lang/ThreadGroup";
    let Value::Ref(group_name) =
        field(vm, groups, group, "name", "Ljava/lang/String;")?
    else {
        unreachable!("a String field")
    };
    Ok(Some((vm.rust_string(name)?, vm.rust_string(group_name)?, priority)))
}
// @end
