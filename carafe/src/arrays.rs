//! Array instructions (JVMS 6.5), and the natives that copy arrays.

use crate::bytecode::{Instruction, Operand};
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::heap::{ARRAY_HEADER, HEADER, Ref, size_of};
use crate::interpreter::{Vm, non_null};
use crate::opcode::*;
use crate::value::Value;

impl Vm {
    // @listing array-op
    /// The instructions that create, measure, read and write arrays.
    pub(crate) fn array_op(&mut self, i: &Instruction) -> Result<()> {
        let from = self.frames.last().expect("a frame").class.clone();
        let value = match (i.opcode, &i.operand) {
            (NEWARRAY, &Operand::ArrayType(code)) => {
                let length = length(self.frame().pop_int()?)?;
                Value::Ref(self.heap.new_array(&primitive(code), length)?)
            }
            (ANEWARRAY, &Operand::Constant(index)) => {
                let name = from.file.constant_pool.class_name(index)?;
                self.resolve_class(&from, name)?;
                let length = length(self.frame().pop_int()?)?;
                Value::Ref(self.heap.new_array(&class_type(name)?, length)?)
            }
            (MULTIANEWARRAY, &Operand::MultiArray { index, dimensions }) => {
                let name = from.file.constant_pool.class_name(index)?;
                self.resolve_class(&from, name)?;
                let mut lengths = vec![0; usize::from(dimensions)];
                for slot in lengths.iter_mut().rev() {
                    *slot = length(self.frame().pop_int()?)?;
                }
                Value::Ref(self.new_multi(&FieldType::parse(name)?, &lengths)?)
            }
            (ARRAYLENGTH, _) => {
                let array = non_null(self.frame().pop_ref()?)?;
                Value::Int(self.heap.array_length(array) as i32)
            }
            (IALOAD..=SALOAD, _) => {
                let index = self.frame().pop_int()?;
                let array = non_null(self.frame().pop_ref()?)?;
                let at = self.heap.element(array, index)?;
                self.heap.read(at, &self.heap.element_type(array).clone())
            }
            _ => {
                // IASTORE..=SASTORE
                let value = self.frame().pop()?;
                let index = self.frame().pop_int()?;
                let array = non_null(self.frame().pop_ref()?)?;
                let at = self.heap.element(array, index)?;
                let ty = self.heap.element_type(array).clone();
                // @hide
                if let Value::Ref(stored) = value
                    && stored != Ref::NULL
                {
                    self.check_store(array, stored)?;
                }
                // @show
                return self.heap.write(at, &ty, value);
            }
        };
        self.frame().push(value)
    }
    // @end

    // @listing new-multi
    /// A multi-dimensional array of type `ty` with these lengths, each
    /// dimension's arrays allocated as elements of the one before. An array
    /// being filled is kept in `heap.roots` while its elements are allocated.
    fn new_multi(&mut self, ty: &FieldType, lengths: &[u32]) -> Result<Ref> {
        let FieldType::Array(element) = ty else { unreachable!("an array type") };
        let array = self.heap.new_array(element, lengths[0])?;
        if lengths.len() == 1 {
            return Ok(array);
        }
        self.heap.roots.push(array);
        let filled = (0..lengths[0]).try_for_each(|i| {
            let inner = self.new_multi(element, &lengths[1..])?;
            let array = *self.heap.roots.last().expect("pushed above");
            let at = self.heap.element(array, i as i32)?;
            self.heap.write(at, element, Value::Ref(inner))
        });
        let array = self.heap.roots.pop().expect("pushed above");
        filled.map(|()| array)
    }

    fn frame(&mut self) -> &mut crate::frame::Frame {
        self.frames.last_mut().expect("a frame")
    }
    // @end
}

// @listing array-helpers
/// An array length from the operand stack, which must not be negative.
pub(crate) fn length(count: i32) -> Result<u32> {
    u32::try_from(count).map_err(|_| Error::Exception {
        class: "java/lang/NegativeArraySizeException",
        message: Some(count.to_string()),
    })
}

/// The element type of `newarray` (JVMS Table 6.5.newarray-A), whose code
/// the decoder has checked is from 4 to 11.
fn primitive(code: u8) -> FieldType {
    match code {
        4 => FieldType::Boolean,
        5 => FieldType::Char,
        6 => FieldType::Float,
        7 => FieldType::Double,
        8 => FieldType::Byte,
        9 => FieldType::Short,
        10 => FieldType::Int,
        11 => FieldType::Long,
        _ => unreachable!("newarray type {code}"),
    }
}

/// The type named by a `Class` constant: a class, or an array class, whose
/// name is its descriptor.
fn class_type(name: &str) -> Result<FieldType> {
    if name.starts_with('[') {
        FieldType::parse(name)
    } else {
        Ok(FieldType::Object(name.to_string()))
    }
}
// @end

// @listing arraycopy
/// `System.arraycopy(src, srcPos, dest, destPos, length)`, with HotSpot's
/// checks and messages. The ranges may overlap, as in `memmove`.
pub(crate) fn arraycopy(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [
        Value::Ref(src),
        Value::Int(src_pos),
        Value::Ref(dest),
        Value::Int(dest_pos),
        Value::Int(length),
    ] = *args
    else {
        unreachable!("checked descriptor")
    };
    let (src, dest) = (non_null(src)?, non_null(dest)?);
    let store = |m: String| Error::Exception {
        class: "java/lang/ArrayStoreException",
        message: Some(format!("arraycopy: {m}")),
    };
    for (r, which) in [(src, "source"), (dest, "destination")] {
        if !vm.heap.is_array(r) {
            let class = vm.heap.class_of(r).file.this_class.replace('/', ".");
            return Err(store(format!("{which} type {class} is not an array")));
        }
    }
    let (from, to) =
        (vm.heap.element_type(src).clone(), vm.heap.element_type(dest).clone());
    let primitive =
        |t: &FieldType| !matches!(t, FieldType::Object(_) | FieldType::Array(_));
    if from != to && (primitive(&from) || primitive(&to)) {
        let name = |t: &FieldType| {
            if primitive(t) { format!("{t}[]") } else { "object array[]".to_string() }
        };
        return Err(store(format!(
            "type mismatch: can not copy {} into {}",
            name(&from),
            name(&to)
        )));
    }
    let bounds = |m: String| Error::Exception {
        class: "java/lang/ArrayIndexOutOfBoundsException",
        message: Some(format!("arraycopy: {m}")),
    };
    // HotSpot names an array of references `object array[5]`, and `int[5]`.
    let ty = |r: Ref, t: &FieldType| match vm.heap.array_length(r) {
        n if primitive(t) => format!("{t}[{n}]"),
        n => format!("object array[{n}]"),
    };
    let (src_ty, dest_ty) = (ty(src, &from), ty(dest, &to));
    if src_pos < 0 {
        return Err(bounds(format!(
            "source index {src_pos} out of bounds for {src_ty}"
        )));
    }
    if dest_pos < 0 {
        let m = format!("destination index {dest_pos} out of bounds for {dest_ty}");
        return Err(bounds(m));
    }
    if length < 0 {
        return Err(bounds(format!("length {length} is negative")));
    }
    for (r, pos, which, ty) in
        [(src, src_pos, "source", src_ty), (dest, dest_pos, "destination", dest_ty)]
    {
        let last = i64::from(pos) + i64::from(length);
        if last > i64::from(vm.heap.array_length(r)) {
            return Err(bounds(format!(
                "last {which} index {last} out of bounds for {ty}"
            )));
        }
    }
    let size = size_of(&from);
    let at = |r: Ref, pos: i32| r.0 + ARRAY_HEADER + pos as u32 * size;
    // @hide
    if !primitive(&from) {
        let storable = vm.storable(src, src_pos, dest, length)?;
        if storable < length {
            vm.heap.copy(
                at(src, src_pos),
                at(dest, dest_pos),
                storable as u32 * size,
            );
            return Err(store(format!(
                "element type mismatch: can not cast one of the elements of {from}[] \
                 to the type of the destination array, {to}"
            )));
        }
    }
    // @show
    vm.heap.copy(at(src, src_pos), at(dest, dest_pos), length as u32 * size);
    Ok(None)
}
// @end

// @listing clone
/// `Object.clone()`: a copy of an array, or of an object whose class
/// implements `Cloneable`, with every field copied and a new identity.
pub(crate) fn clone(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(this)] = *args else { unreachable!("checked descriptor") };
    let class = vm.class_of(this)?;
    let copy = if vm.heap.is_array(this) {
        let (ty, length) =
            (vm.heap.element_type(this).clone(), vm.heap.array_length(this));
        vm.heap.roots.push(this);
        let copy = vm.heap.new_array(&ty, length);
        let this = vm.heap.roots.pop().expect("pushed above");
        let copy = copy?;
        let bytes = length * size_of(&ty);
        vm.heap.copy(this.0 + ARRAY_HEADER, copy.0 + ARRAY_HEADER, bytes);
        copy
    } else {
        if !implements(&class, "java/lang/Cloneable") {
            let class = "java/lang/CloneNotSupportedException";
            let name = vm.heap.class_of(this).file.this_class.replace('/', ".");
            return Err(Error::Exception { class, message: Some(name) });
        }
        vm.heap.roots.push(this);
        let copy = vm.heap.new_object(&class);
        let this = vm.heap.roots.pop().expect("pushed above");
        vm.heap.copy(this.0 + HEADER, copy.0 + HEADER, class.layout.size - HEADER);
        copy
    };
    Ok(Some(Value::Ref(copy)))
}

/// Whether `class` or a superclass implements the interface `name`,
/// directly or through a superinterface.
fn implements(class: &crate::class::Class, name: &str) -> bool {
    class.interfaces.iter().any(|i| i.file.this_class == name || implements(i, name))
        || class.superclass.as_ref().is_some_and(|s| implements(s, name))
}
// @end
