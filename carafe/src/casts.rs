//! Assignability, `checkcast` and `instanceof` (JVMS 6.5).

use crate::class::Class;
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::opcode::INSTANCEOF;
use crate::value::Value;
use std::rc::Rc;

impl Vm {
    // @listing assignable
    /// Whether a value of class `s` may be used where class `t` is expected:
    /// the rules of `checkcast` (JVMS 6.5), for classes, interfaces and arrays.
    pub fn is_assignable(&mut self, s: &Rc<Class>, t: &Rc<Class>) -> Result<bool> {
        if Rc::ptr_eq(s, t) {
            return Ok(true);
        }
        let (s_name, t_name) = (s.file.this_class.clone(), t.file.this_class.clone());
        match (s_name.strip_prefix('['), t_name.strip_prefix('[')) {
            // Arrays: the same primitive element type, or element types
            // that are references and assignable themselves.
            (Some(sc), Some(tc)) => match (reference(sc), reference(tc)) {
                (Some(sc), Some(tc)) => {
                    let (sc, tc) = (self.load(sc)?, self.load(tc)?);
                    self.is_assignable(&sc, &tc)
                }
                _ => Ok(sc == tc),
            },
            (Some(_), None) => Ok(matches!(
                t_name.as_str(),
                "java/lang/Object" | "java/lang/Cloneable" | "java/io/Serializable"
            )),
            (None, Some(_)) => Ok(false),
            (None, None) => Ok(is_subtype(s, t)),
        }
    }
    // @end

    // @listing type-check
    /// `checkcast` and `instanceof`. `null` passes `checkcast` and is an
    /// instance of nothing.
    pub(crate) fn type_check(&mut self, opcode: u8, index: u16) -> Result<()> {
        let from = self.frames.last().expect("a frame").class.clone();
        let target =
            self.resolve_class(&from, from.file.constant_pool.class_name(index)?)?;
        let object = self.frames.last_mut().expect("a frame").pop_ref()?;
        let fits = object == Ref::NULL || {
            let class = self.class_of(object)?;
            self.is_assignable(&class, &target)?
        };
        let frame = self.frames.last_mut().expect("a frame");
        if opcode == INSTANCEOF {
            return frame.push(Value::Int((fits && object != Ref::NULL) as i32));
        }
        if !fits {
            let class = self.class_of(object)?;
            let m = format!(
                "class {} cannot be cast to class {} ({})",
                java_name(&class),
                java_name(&target),
                modules(&class, &target)
            );
            let class = "java/lang/ClassCastException";
            return Err(Error::Exception { class, message: Some(m) });
        }
        self.frames.last_mut().expect("a frame").push(Value::Ref(object))
    }
    // @end
}

// @listing subtype
/// Whether class or interface `s` is `t`, or extends or implements it,
/// directly or through its superclasses and superinterfaces.
pub(crate) fn is_subtype(s: &Rc<Class>, t: &Rc<Class>) -> bool {
    Rc::ptr_eq(s, t)
        || s.superclass.as_ref().is_some_and(|c| is_subtype(c, t))
        || s.interfaces.iter().any(|i| is_subtype(i, t))
}

/// The class name in an array's element descriptor, if the element is a
/// reference: `Ljava/lang/String;` names `java/lang/String`, `[I` itself.
fn reference(descriptor: &str) -> Option<&str> {
    match descriptor.as_bytes().first()? {
        b'L' => descriptor.get(1..descriptor.len() - 1),
        b'[' => Some(descriptor),
        _ => None,
    }
}
// @end

// @listing names
/// A class's name as Java's `Class.getName` gives it: `java.lang.String`,
/// and for an array its descriptor with dots, `[Ljava.lang.String;`.
pub(crate) fn java_name(class: &Class) -> String {
    class.file.this_class.replace('/', ".")
}

/// HotSpot's note on where two classes come from: the program's classes are
/// in the unnamed module of the application class loader, and `java.base`'s
/// in module `java.base` of the bootstrap loader.
pub(crate) fn modules(s: &Class, t: &Class) -> String {
    let place = |c: &Class| match c.boot {
        true => "module java.base of loader 'bootstrap'",
        false => "unnamed module of loader 'app'",
    };
    match place(s) == place(t) {
        true => format!("{} and {} are in {}", java_name(s), java_name(t), place(s)),
        false => format!(
            "{} is in {}; {} is in {}",
            java_name(s),
            place(s),
            java_name(t),
            place(t)
        ),
    }
}
// @end

impl Vm {
    // @listing check-store
    /// The check `aastore` makes before it stores the non-null `value` into
    /// `array`: the value's class must be assignable to the element type.
    pub(crate) fn check_store(&mut self, array: Ref, value: Ref) -> Result<()> {
        let element = self.element_class(array)?;
        let class = self.class_of(value)?;
        if self.is_assignable(&class, &element)? {
            return Ok(());
        }
        let class_name = java_name(&class);
        let class = "java/lang/ArrayStoreException";
        Err(Error::Exception { class, message: Some(class_name) })
    }

    /// How many of the `length` elements from `src_pos` in `src` can be
    /// stored in `dest`: all of them if the arrays' element types allow it,
    /// otherwise the number before the first that can't.
    pub(crate) fn storable(
        &mut self,
        src: Ref,
        src_pos: i32,
        dest: Ref,
        length: i32,
    ) -> Result<i32> {
        let (source, element) = (self.class_of(src)?, self.element_class(dest)?);
        let destination = self.class_of(dest)?;
        if self.is_assignable(&source, &destination)? {
            return Ok(length);
        }
        for i in 0..length {
            let at = self.heap.element(src, src_pos + i)?;
            let ty = self.heap.element_type(src).clone();
            let Value::Ref(value) = self.heap.read(at, &ty) else { unreachable!() };
            if value != Ref::NULL {
                let class = self.class_of(value)?;
                if !self.is_assignable(&class, &element)? {
                    return Ok(i);
                }
            }
        }
        Ok(length)
    }

    /// The class of the elements of the reference array at `array`.
    fn element_class(&mut self, array: Ref) -> Result<Rc<Class>> {
        let class = self.class_of(array)?;
        let component = &class.file.this_class[1..];
        let name = reference(component).expect("an array of references");
        self.load(name)
    }
    // @end
}
