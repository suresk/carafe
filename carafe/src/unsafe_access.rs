//! `jdk.internal.misc.Unsafe`: the JDK's access to memory by offset.

use crate::class::Slot;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::heap::{ARRAY_HEADER, Ref, size_of};
use crate::interpreter::{Vm, non_null};
use crate::natives::Native;
use crate::value::Value;

// @listing unsafe-offsets
/// `Unsafe.arrayBaseOffset0(Class)`: where an array's elements start, the
/// same for every array in carafe.
fn array_base_offset(_: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    Ok(Some(Value::Int(ARRAY_HEADER as i32)))
}

/// `Unsafe.arrayIndexScale0(Class)`: the bytes between an array's elements.
fn array_index_scale(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [_, Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let class = vm.class_of_mirror(mirror)?;
    let element = FieldType::parse(&class.file.this_class)?;
    let FieldType::Array(element) = element else {
        return Err(Error::Verify(format!(
            "{} is not an array",
            class.file.this_class
        )));
    };
    Ok(Some(Value::Int(size_of(&element) as i32)))
}

/// `Unsafe.objectFieldOffset1(Class, String)`: the offset of an instance
/// field from the start of the object, which is where carafe's layout put it.
fn object_field_offset(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [_, Value::Ref(mirror), Value::Ref(name)] = *args else { unreachable!() };
    let class = vm.class_of_mirror(mirror)?;
    let name = vm.rust_string(name)?;
    let Some(slot) = class.layout.fields.iter().rev().find(|f| f.name == name) else {
        let class = "java/lang/InternalError";
        return Err(Error::Exception { class, message: Some(name) });
    };
    Ok(Some(Value::Long(slot.offset.into())))
}

// @hide
/// `Unsafe.objectFieldOffset0(Field)`: the same, for the field a `Field`
/// stands for, by its declaring class and its slot.
fn field_offset(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [_, Value::Ref(field)] = *args else { unreachable!("checked descriptor") };
    let (class, index) = vm.member_of(field, "java/lang/reflect/Field")?;
    let declared = &class.file.fields[index];
    let same =
        |f: &&Slot| f.name == declared.name && f.descriptor == declared.descriptor;
    let found = class.layout.fields.iter().rev().find(same);
    let Some(slot) = found else {
        let class = "java/lang/InternalError";
        return Err(Error::Exception { class, message: Some(declared.name.clone()) });
    };
    Ok(Some(Value::Long(slot.offset.into())))
}
// @show
// @end

// @listing unsafe-access
/// `Unsafe.getInt(Object, long)` and the rest: the value of type `ty` at
/// `offset` bytes into the object. carafe has no memory outside the heap, so
/// the object can't be `null`.
pub(crate) fn get(
    vm: &mut Vm,
    args: &[Value],
    ty: FieldType,
) -> Result<Option<Value>> {
    let [_, Value::Ref(object), Value::Long(offset), ..] = *args else {
        unreachable!()
    };
    let at = address(object, offset)?;
    Ok(Some(vm.heap.read(at, &ty)))
}

/// `Unsafe.putInt(Object, long, int)` and the rest.
pub(crate) fn put(
    vm: &mut Vm,
    args: &[Value],
    ty: FieldType,
) -> Result<Option<Value>> {
    let [_, Value::Ref(object), Value::Long(offset), value] = *args else {
        unreachable!()
    };
    vm.heap.write(address(object, offset)?, &ty, value)?;
    Ok(None)
}

/// `Unsafe.compareAndSetInt(Object, long, int expected, int value)` and the
/// rest: store `value` if the field holds `expected`. With one thread there
/// is nothing to make atomic.
pub(crate) fn compare_and_set(
    vm: &mut Vm,
    args: &[Value],
    ty: FieldType,
) -> Result<Option<Value>> {
    let [_, Value::Ref(object), Value::Long(offset), expected, value] = *args else {
        unreachable!("checked descriptor")
    };
    let at = address(object, offset)?;
    let same = vm.heap.read(at, &ty) == expected;
    if same {
        vm.heap.write(at, &ty, value)?;
    }
    Ok(Some(Value::Int(same as i32)))
}

/// The type `Object`, for the accessors of references.
pub(crate) fn object() -> FieldType {
    FieldType::Object("java/lang/Object".to_string())
}

/// The heap offset `offset` bytes into `object`.
fn address(object: Ref, offset: i64) -> Result<u32> {
    let object = non_null(object)?;
    u32::try_from(i64::from(object.0) + offset)
        .map_err(|_| Error::Unsupported(format!("Unsafe at offset {offset}")))
}
// @end

// @listing unsafe-find
/// Writes a pair of natives for a type: `getX` and `putX`.
macro_rules! accessors {
    ($($get:ident $put:ident $ty:expr;)*) => {
        $(
            fn $get(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
                get(vm, args, $ty)
            }
            fn $put(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
                put(vm, args, $ty)
            }
        )*
    };
}

accessors! {
    get_int put_int FieldType::Int;
    get_long put_long FieldType::Long;
    get_byte put_byte FieldType::Byte;
    get_short put_short FieldType::Short;
    get_char put_char FieldType::Char;
    get_boolean put_boolean FieldType::Boolean;
    get_float put_float FieldType::Float;
    get_double put_double FieldType::Double;
    get_reference put_reference object();
}

/// The native `Unsafe.name`, which is unique by name among the natives
/// carafe implements. The `Volatile` forms of the accessors are the plain
/// ones: with one thread, every access is already in program order.
pub(crate) fn find(name: &str) -> Option<Native> {
    Some(match name.strip_suffix("Volatile").unwrap_or(name) {
        "getInt" => get_int,
        "putInt" => put_int,
        "getLong" => get_long,
        "putLong" => put_long,
        "getByte" => get_byte,
        "putByte" => put_byte,
        "getShort" => get_short,
        "putShort" => put_short,
        "getChar" => get_char,
        "putChar" => put_char,
        "getBoolean" => get_boolean,
        "putBoolean" => put_boolean,
        "getFloat" => get_float,
        "putFloat" => put_float,
        "getDouble" => get_double,
        "putDouble" => put_double,
        "getReference" => get_reference,
        "putReference" => put_reference,
        "compareAndSetInt" => |vm, args| compare_and_set(vm, args, FieldType::Int),
        "compareAndSetLong" => |vm, args| compare_and_set(vm, args, FieldType::Long),
        "compareAndSetReference" => |vm, args| compare_and_set(vm, args, object()),
        "arrayBaseOffset0" => array_base_offset,
        "arrayIndexScale0" => array_index_scale,
        "objectFieldOffset1" => object_field_offset,
        // @hide
        "objectFieldOffset0" => field_offset,
        // @show
        "registerNatives" | "fullFence" => crate::lang::nothing,
        _ => return None,
    })
}
// @end
