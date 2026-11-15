//! Natives of `java.lang` and of the JDK's internal startup classes.

use crate::error::Result;
use crate::interpreter::Vm;
use crate::value::Value;

// @listing primitive-class
/// `Class.getPrimitiveClass(String)`: the `Class` object of a primitive type,
/// such as `Integer.TYPE` for `int`; one per name, as for any class.
pub(crate) fn get_primitive_class(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(name)] = *args else { unreachable!("checked descriptor") };
    let name = vm.rust_string(name)?;
    if let Some(&mirror) = vm.primitive_mirrors.get(&name) {
        return Ok(Some(Value::Ref(mirror)));
    }
    let class_class = vm.load("java/lang/Class")?;
    let mirror = vm.heap.new_object(&class_class);
    vm.primitive_mirrors.insert(name, mirror);
    Ok(Some(Value::Ref(mirror)))
}
// @end

// @listing bits
/// `Float.floatToRawIntBits` and `intBitsToFloat`, and the same for
/// `double`: the bits of a floating-point number, unchanged, as an integer.
pub(crate) fn float_to_bits(_: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Float(f)] = *args else { unreachable!("checked descriptor") };
    Ok(Some(Value::Int(f.to_bits() as i32)))
}

pub(crate) fn bits_to_float(_: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Int(i)] = *args else { unreachable!("checked descriptor") };
    Ok(Some(Value::Float(f32::from_bits(i as u32))))
}

pub(crate) fn double_to_bits(_: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Double(d)] = *args else { unreachable!("checked descriptor") };
    Ok(Some(Value::Long(d.to_bits() as i64)))
}

pub(crate) fn bits_to_double(_: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Long(l)] = *args else { unreachable!("checked descriptor") };
    Ok(Some(Value::Double(f64::from_bits(l as u64))))
}
// @end

// @listing max-memory
/// `Runtime.maxMemory()`: the most heap carafe can address with 32-bit
/// references, 4 GB.
pub(crate) fn max_memory(_: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    Ok(Some(Value::Long(1 << 32)))
}
// @end

// @listing stubs
/// A native that does nothing: `registerNatives`, which binds natives in
/// HotSpot and has nothing to bind in carafe, and `VM.initialize`.
pub(crate) fn nothing(_: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    Ok(None)
}

/// A native that returns `false` or 0, the answer that turns a feature off:
/// `CDS.getCDSConfigStatus`, which says that no class data archive is in use.
pub(crate) fn zero(_: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    Ok(Some(Value::Int(0)))
}
// @end
