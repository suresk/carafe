//! `ObjectMethods.bootstrap`: a record's `toString`, `equals` and `hashCode`.

use crate::class::{Class, Slot};
use crate::descriptor::FieldType;
use crate::dynamic::CallSite;
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::pool::Constant;
use crate::reader::Reader;
use crate::value::Value;
use std::rc::Rc;

// @listing record-site
/// Which of a record's methods a call site of `ObjectMethods.bootstrap` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordMethod {
    ToString,
    Equals,
    HashCode,
}

impl Vm {
    /// Links a call site of `ObjectMethods.bootstrap`. The name says which
    /// method; the static arguments are the record class, its components'
    /// names joined by `;`, and a `getField` handle for each component.
    pub(crate) fn record_site(
        &mut self,
        caller: &Class,
        name: &str,
        statics: &[u16],
    ) -> Result<CallSite> {
        let pool = &caller.file.constant_pool;
        let method = match name {
            "toString" => RecordMethod::ToString,
            "equals" => RecordMethod::Equals,
            "hashCode" => RecordMethod::HashCode,
            other => {
                let m = format!("Illegal method name {other}");
                return Err(linkage("java/lang/BootstrapMethodError", m));
            }
        };
        let Some((&record, [_, handles @ ..])) = statics.split_first() else {
            return Err(linkage(
                "java/lang/BootstrapMethodError",
                "no record".into(),
            ));
        };
        let class = self.resolve_class(caller, pool.class_name(record)?)?;
        let mut components = Vec::new();
        for &handle in handles {
            let &Constant::MethodHandle { reference, .. } = pool.get(handle)? else {
                return Err(Error::ClassFormat(format!(
                    "#{handle} is not a MethodHandle"
                )));
            };
            let field = pool.member(reference)?;
            components.push(class.field(field.name, field.descriptor)?.clone());
        }
        Ok(CallSite::Record { method, class, components })
    }
    // @end

    // @listing record-method
    /// Runs a record method. The record, and for `equals` the other object,
    /// stay on the operand stack, as roots, until the result is made.
    pub(crate) fn record_method(
        &mut self,
        method: RecordMethod,
        class: &Rc<Class>,
        components: &[Slot],
    ) -> Result<()> {
        let at = self.frames.len() - 1;
        let args = if method == RecordMethod::Equals { 2 } else { 1 };
        let Some(base) = self.frames[at].stack.len().checked_sub(args) else {
            return Err(Error::Verify("invokedynamic underflows the stack".into()));
        };
        // A component of the record at `slot` on the stack, read afresh.
        let read = |vm: &Vm, at_slot: usize, c: &Slot| {
            let Value::Ref(r) = vm.frames[at].stack[at_slot] else {
                unreachable!("a record")
            };
            vm.heap.read(r.0 + c.offset, &c.ty)
        };
        let result = match method {
            RecordMethod::ToString => {
                let mut units: Vec<u16> =
                    simple_name(class)?.encode_utf16().collect();
                units.push(u16::from(b'['));
                for (i, c) in components.iter().enumerate() {
                    let separator = if i == 0 { "" } else { ", " };
                    units.extend(format!("{separator}{}=", c.name).encode_utf16());
                    let value = read(self, base, c);
                    units.extend(self.value_of(&c.ty, value)?);
                }
                units.push(u16::from(b']'));
                Value::Ref(self.new_string(&units)?)
            }
            RecordMethod::Equals => {
                let Value::Ref(other) = self.frames[at].stack[base + 1] else {
                    unreachable!("an Object")
                };
                let mut equal = other != Ref::NULL && {
                    let other = self.class_of(other)?;
                    self.is_assignable(&other, class)?
                };
                for c in components {
                    if !equal {
                        break;
                    }
                    let (a, b) = (read(self, base, c), read(self, base + 1, c));
                    equal = match (a, b) {
                        (Value::Float(a), Value::Float(b)) => {
                            same_float(a.into(), b.into())
                        }
                        (Value::Double(a), Value::Double(b)) => same_float(a, b),
                        (Value::Ref(_), Value::Ref(_)) => {
                            let objects = self.load("java/util/Objects")?;
                            let descriptor =
                                "(Ljava/lang/Object;Ljava/lang/Object;)Z";
                            let equals =
                                self.invoke(&objects, "equals", descriptor, &[a, b])?;
                            equals == Some(Value::Int(1))
                        }
                        (a, b) => a == b,
                    };
                }
                Value::Int(equal.into())
            }
            RecordMethod::HashCode => {
                let mut hash = 0i32;
                for c in components {
                    let value = read(self, base, c);
                    let h = self.hash_of(&c.ty, value)?;
                    hash = hash.wrapping_mul(31).wrapping_add(h);
                }
                Value::Int(hash)
            }
        };
        self.frames[at].stack.truncate(base);
        self.frames[at].push(result)
    }
    // @end

    // @listing hash-of
    /// A component's hash code, as its wrapper's static `hashCode` computes
    /// it, or `Objects.hashCode` for a reference.
    fn hash_of(&mut self, ty: &FieldType, value: Value) -> Result<i32> {
        Ok(match (ty, value) {
            (FieldType::Boolean, Value::Int(v)) => {
                if v != 0 {
                    1231
                } else {
                    1237
                }
            }
            (_, Value::Int(v)) => v,
            (_, Value::Long(v)) => (v ^ (v >> 32)) as i32,
            (_, Value::Float(v)) => {
                if v.is_nan() {
                    0x7fc0_0000
                } else {
                    v.to_bits() as i32
                }
            }
            (_, Value::Double(v)) => {
                let bits =
                    if v.is_nan() { 0x7ff8_0000_0000_0000 } else { v.to_bits() };
                (bits ^ (bits >> 32)) as i32
            }
            (_, Value::Ref(_)) => {
                let objects = self.load("java/util/Objects")?;
                let hash = self.invoke(
                    &objects,
                    "hashCode",
                    "(Ljava/lang/Object;)I",
                    &[value],
                )?;
                let Some(Value::Int(h)) = hash else { unreachable!("an int") };
                h
            }
            (_, Value::Top) => unreachable!("a component's value"),
        })
    }
    // @end
}

/// `Float.compare(a, b) == 0`: the same bits, or both NaN.
fn same_float(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
}

// @listing simple-name
/// `Class.getSimpleName()`: a nested class's name as its source declares
/// it, from the `InnerClasses` attribute, or a top-level class's name
/// without its package.
pub(crate) fn simple_name(class: &Class) -> Result<String> {
    let (name, pool) = (&class.file.this_class, &class.file.constant_pool);
    if let Some(inner) =
        class.file.attributes.iter().find(|a| a.name == "InnerClasses")
    {
        let mut r = Reader::new(&inner.info);
        for _ in 0..r.u2()? {
            let (inner, _outer, simple, _flags) =
                (r.u2()?, r.u2()?, r.u2()?, r.u2()?);
            if pool.class_name(inner)? == name {
                return Ok(match simple {
                    0 => String::new(), // an anonymous class
                    _ => pool.utf8(simple)?.to_string(),
                });
            }
        }
    }
    Ok(name.rsplit('/').next().unwrap_or(name).to_string())
}
// @end
