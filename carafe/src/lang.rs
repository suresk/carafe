//! Natives of `java.lang` and of the JDK's internal startup classes.

use crate::class::Slot;
use crate::error::Result;
use crate::heap::Ref;
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
    Ok(Some(Value::Ref(vm.primitive_mirror(&name)?)))
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

// @listing clocks
/// `System.nanoTime()`: carafe's clock counts instructions executed, so a
/// program that seeds something with the time does the same on every run.
pub(crate) fn nano_time(vm: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    Ok(Some(Value::Long(vm.executed as i64)))
}

/// `System.currentTimeMillis()`: the time of day, which can't be the same
/// on every run, in milliseconds since 1970.
pub(crate) fn current_time_millis(_: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    Ok(Some(Value::Long(now.map_or(0, |d| d.as_millis() as i64))))
}
// @end

// @listing references
/// `Reference.get0()`, `refersTo0(Object)` and `clear0()`: soft, weak and
/// phantom references, which a collector clears when nothing else refers to
/// their object. carafe has no collector until JVM2, so a reference keeps
/// its object until the program clears it.
fn referent(vm: &mut Vm) -> Result<Slot> {
    let class = vm.load("java/lang/ref/Reference")?;
    Ok(class.field("referent", "Ljava/lang/Object;")?.clone())
}

pub(crate) fn reference_get(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(this)] = *args else { unreachable!("checked descriptor") };
    let slot = referent(vm)?;
    Ok(Some(vm.heap.read(this.0 + slot.offset, &slot.ty)))
}

pub(crate) fn refers_to(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(this), Value::Ref(object)] = *args else { unreachable!() };
    let slot = referent(vm)?;
    let same = vm.heap.read(this.0 + slot.offset, &slot.ty) == Value::Ref(object);
    Ok(Some(Value::Int(same.into())))
}

pub(crate) fn reference_clear(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(this)] = *args else { unreachable!("checked descriptor") };
    let slot = referent(vm)?;
    vm.heap.write(this.0 + slot.offset, &slot.ty, Value::Ref(Ref::NULL))?;
    Ok(None)
}
// @end
