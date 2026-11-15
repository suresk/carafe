//! `java.lang.Class` objects.

use crate::casts::java_name;
use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::pool::Constant;
use crate::reader::Reader;
use crate::value::Value;
use std::rc::Rc;

impl Vm {
    // @listing mirror
    /// The `java.lang.Class` object for `class`: one per class, created the
    /// first time something asks for it, with the fields `Class`'s own methods
    /// read: the modifiers, and an array's component type.
    pub fn mirror(&mut self, class: &Rc<Class>) -> Result<Ref> {
        if class.mirror.get() != Ref::NULL {
            return Ok(class.mirror.get());
        }
        let component = match class.file.this_class.strip_prefix('[') {
            Some(element) => self.component_mirror(element)?,
            None => Ref::NULL,
        };
        self.heap.roots.push(component); // held across the next allocation
        let class_class = self.load("java/lang/Class")?;
        let mirror = self.heap.new_object(&class_class);
        let component = self.heap.roots.pop().expect("pushed above");
        let modifiers = Value::Int(modifiers(self, class)?);
        self.put(&class_class, mirror, "modifiers", "C", modifiers)?;
        let component = Value::Ref(component);
        self.put(
            &class_class,
            mirror,
            "componentType",
            "Ljava/lang/Class;",
            component,
        )?;
        class.mirror.set(mirror);
        self.mirror_classes.insert(mirror, class.clone());
        Ok(mirror)
    }

    /// The mirror of an array's component type, from its descriptor.
    fn component_mirror(&mut self, descriptor: &str) -> Result<Ref> {
        match FieldType::parse(descriptor)? {
            FieldType::Object(name) => {
                let class = self.load(&name)?;
                self.mirror(&class)
            }
            FieldType::Array(_) => {
                let class = self.load(descriptor)?;
                self.mirror(&class)
            }
            primitive => self.primitive_mirror(&primitive.to_string()),
        }
    }

    /// The mirror of the primitive type `name`, such as `int`.
    pub(crate) fn primitive_mirror(&mut self, name: &str) -> Result<Ref> {
        if let Some(&mirror) = self.primitive_mirrors.get(name) {
            return Ok(mirror);
        }
        let class_class = self.load("java/lang/Class")?;
        let mirror = self.heap.new_object(&class_class);
        let flags = AccessFlags::PUBLIC | AccessFlags::FINAL | AccessFlags::ABSTRACT;
        self.put(&class_class, mirror, "modifiers", "C", Value::Int(flags.into()))?;
        self.put(&class_class, mirror, "primitive", "Z", Value::Int(1))?;
        self.primitive_mirrors.insert(name.to_string(), mirror);
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

// @listing class-name
/// `Class.initClassName()`: the class's name as `Class.getName` returns it,
/// `java.lang.String` or `[I` or `int`, stored in the mirror's `name` field
/// so that later calls don't come back here.
pub(crate) fn init_class_name(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let name = match vm.mirror_classes.get(&mirror) {
        Some(class) => java_name(class),
        None => {
            let primitive = vm.primitive_mirrors.iter().find(|&(_, &m)| m == mirror);
            primitive.map(|(name, _)| name.clone()).expect("a mirror")
        }
    };
    let units: Vec<u16> = name.encode_utf16().collect();
    let name = vm.intern(&units)?;
    let class_class = vm.heap.class_of(mirror).clone();
    let slot = class_class.field("name", "Ljava/lang/String;")?;
    vm.heap.write(mirror.0 + slot.offset, &slot.ty, Value::Ref(name))?;
    Ok(Some(Value::Ref(name)))
}
// @end

// @listing hash-code
/// `Object.hashCode()` and `System.identityHashCode(Object)`: the identity
/// hash in the object's header, its first address; 0 for `null`.
pub(crate) fn identity_hash(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(object)] = *args else { unreachable!("checked descriptor") };
    let hash = if object == Ref::NULL { 0 } else { vm.heap.hash(object) as i32 };
    Ok(Some(Value::Int(hash)))
}
// @end

// @listing modifiers
/// The modifiers `Class.getModifiers` reports: a nested class's from its
/// `InnerClasses` entry, which has `static`, `private` and `protected`, a
/// top-level class's from its access flags; an array's are its component's
/// access, `final` and `abstract`.
fn modifiers(vm: &mut Vm, class: &Class) -> Result<i32> {
    let name = &class.file.this_class;
    let access = AccessFlags::PUBLIC | AccessFlags::PRIVATE | AccessFlags::PROTECTED;
    if let Some(element) = name.strip_prefix('[') {
        let component = match FieldType::parse(element)? {
            FieldType::Object(c) => vm.load(&c)?.file.access_flags.0,
            FieldType::Array(_) => vm.load(element)?.file.access_flags.0,
            _ => AccessFlags::PUBLIC,
        };
        let flags = (component & access) | AccessFlags::FINAL | AccessFlags::ABSTRACT;
        return Ok(flags.into());
    }
    if let Some(inner) =
        class.file.attributes.iter().find(|a| a.name == "InnerClasses")
    {
        let mut r = Reader::new(&inner.info);
        for _ in 0..r.u2()? {
            let (inner_class, _, _, flags) = (r.u2()?, r.u2()?, r.u2()?, r.u2()?);
            if class.file.constant_pool.class_name(inner_class)? == name {
                return Ok((flags & !AccessFlags::SUPER).into());
            }
        }
    }
    Ok((class.file.access_flags.0 & !(AccessFlags::SUPER | AccessFlags::SYNTHETIC))
        .into())
}
// @end

// @listing class-natives
/// `Class.isInstance(Object)`: whether the object is an instance of the class.
pub(crate) fn is_instance(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(mirror), Value::Ref(object)] = *args else { unreachable!() };
    if object == Ref::NULL || vm.primitive_mirrors.values().any(|&m| m == mirror) {
        return Ok(Some(Value::Int(0)));
    }
    let (class, target) = (vm.class_of(object)?, vm.class_of_mirror(mirror)?);
    Ok(Some(Value::Int(vm.is_assignable(&class, &target)? as i32)))
}

/// `Class.isAssignableFrom(Class)`: whether a value of the argument's class
/// can be used where the receiver's is expected. A primitive class is
/// assignable only from itself.
pub(crate) fn is_assignable_from(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(target), Value::Ref(source)] = *args else { unreachable!() };
    let source = crate::interpreter::non_null(source)?;
    let primitive = |vm: &Vm, m: Ref| vm.primitive_mirrors.values().any(|&p| p == m);
    if primitive(vm, target) || primitive(vm, source) {
        return Ok(Some(Value::Int((target == source) as i32)));
    }
    let (target, source) = (vm.class_of_mirror(target)?, vm.class_of_mirror(source)?);
    Ok(Some(Value::Int(vm.is_assignable(&source, &target)? as i32)))
}

/// `Class.getSuperclass()`: `null` for `Object`, an interface or a primitive
/// type, `Object` for an array, and otherwise the superclass.
pub(crate) fn get_superclass(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let Ok(class) = vm.class_of_mirror(mirror) else {
        return Ok(Some(Value::Ref(Ref::NULL))); // a primitive type
    };
    let superclass = match class.file.access_flags.has(AccessFlags::INTERFACE) {
        true => None,
        false => class.superclass.clone(),
    };
    let superclass = match superclass {
        Some(superclass) => vm.mirror(&superclass)?,
        None => Ref::NULL,
    };
    Ok(Some(Value::Ref(superclass)))
}
// @end
