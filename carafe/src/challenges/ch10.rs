//! Chapter 10: strings, the shims and Hello, world.

use crate::class::State;
use crate::descriptor::FieldType;
use crate::error::Result;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::strings::LATIN1;
use crate::value::Value;

// @listing initialized
/// The classes `vm` has loaded and initialized, in the order they loaded.
pub fn initialized(vm: &mut Vm) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for name in vm.loaded.clone() {
        if vm.load(&name)?.state.get() == State::Initialized {
            found.push(name);
        }
    }
    Ok(found)
}
// @end

// @listing java-hash
/// `String.hashCode` computed in Rust from the string's bytes: the sum of
/// each UTF-16 code unit times 31 to the power of the units after it.
pub fn java_hash(vm: &Vm, string: Ref) -> Result<i32> {
    let class = vm.heap.class_of(string);
    let read = |name: &str, ty: &str| -> Result<Value> {
        let slot = class.field(name, ty)?;
        Ok(vm.heap.read(string.0 + slot.offset, &slot.ty))
    };
    let (Value::Ref(value), Value::Int(coder)) =
        (read("value", "[B")?, read("coder", "B")?)
    else {
        unreachable!("a String")
    };
    let byte = |i: u32| -> Result<u8> {
        let at = vm.heap.element(value, i as i32)?;
        match vm.heap.read(at, &FieldType::Byte) {
            Value::Int(b) => Ok(b as u8),
            _ => unreachable!(),
        }
    };
    let length = vm.heap.array_length(value);
    let mut hash: i32 = 0;
    let (units, width) = if coder == LATIN1 { (length, 1) } else { (length / 2, 2) };
    for i in 0..units {
        let unit = match width {
            1 => u16::from(byte(i)?),
            _ => u16::from_le_bytes([byte(2 * i)?, byte(2 * i + 1)?]),
        };
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    Ok(hash)
}
// @end
