//! `java.lang.Class` objects.

use crate::class::Class;
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::pool::Constant;
use crate::value::Value;
use std::rc::Rc;

impl Vm {
    // @listing mirror
    /// The `java.lang.Class` object for `class`: one per class, created the
    /// first time something asks for it. Chapter 15 gives it its name.
    pub fn mirror(&mut self, class: &Rc<Class>) -> Result<Ref> {
        if class.mirror.get() != Ref::NULL {
            return Ok(class.mirror.get());
        }
        let class_class = self.load("java/lang/Class")?;
        let mirror = self.heap.new_object(&class_class);
        class.mirror.set(mirror);
        // @hide
        self.mirror_classes.insert(mirror, class.clone());
        // @show
        Ok(mirror)
    }

    /// `ldc` of a `Class` constant: the class, resolved, and its mirror.
    pub(crate) fn ldc_class(&mut self, index: u16) -> Result<Ref> {
        let from = self.frames.last().expect("a frame").class.clone();
        let name = from.file.constant_pool.class_name(index)?;
        let class = self.resolve_class(&from, name)?;
        self.mirror(&class)
    }
    // @end
}

impl Vm {
    // @listing class-of-mirror
    /// The class a `java.lang.Class` object stands for.
    pub fn class_of_mirror(&self, mirror: Ref) -> Result<Rc<Class>> {
        match self.mirror_classes.get(&mirror) {
            Some(class) => Ok(class.clone()),
            None => {
                Err(Error::Unsupported("the class of a primitive type".to_string()))
            }
        }
    }
    // @end
}

// @listing is-class
/// Whether the constant at `index` in `class`'s pool is a `Class`.
pub(crate) fn is_class(class: &Class, index: u16) -> bool {
    matches!(class.file.constant_pool.get(index), Ok(Constant::Class { .. }))
}

/// `Class.registerNatives()`: HotSpot binds `Class`'s natives here; carafe
/// finds natives by name, so there is nothing to do.
pub(crate) fn register_natives(_: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    Ok(None)
}

/// `Class.desiredAssertionStatus0(Class)`: whether `assert` statements in the
/// class run. They don't, as with `java` run without `-ea`.
pub(crate) fn desired_assertion_status(
    _: &mut Vm,
    _: &[Value],
) -> Result<Option<Value>> {
    Ok(Some(Value::Int(0)))
}
// @end

// @listing get-class
/// `Object.getClass()`: the mirror of the object's class, or of an array's.
pub(crate) fn get_class(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(this)] = *args else { unreachable!("checked descriptor") };
    let class = vm.class_of(this)?;
    Ok(Some(Value::Ref(vm.mirror(&class)?)))
}
// @end
