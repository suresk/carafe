//! Just enough reflection: `Method` objects, and calling a method through one.

use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::frame::Frame;
use crate::heap::Ref;
use crate::interpreter::{Vm, non_null, select};
use crate::lambda::wrapper;
use crate::reader::Reader;
use crate::value::Value;
use std::rc::Rc;

// @listing declared-methods
/// `Class.getDeclaredMethods0(boolean publicOnly)`: a `Method` for each
/// method the class declares, other than constructors and `<clinit>`.
pub(crate) fn get_declared_methods(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    declared(vm, args, Member::Method)
}

/// `Class.getDeclaredConstructors0(boolean publicOnly)`: a `Constructor` for
/// each of the class's constructors.
pub(crate) fn get_declared_constructors(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    declared(vm, args, Member::Constructor)
}

/// Which kind of member a reflective object stands for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Member {
    Method,
    Constructor,
}

fn declared(vm: &mut Vm, args: &[Value], kind: Member) -> Result<Option<Value>> {
    let [Value::Ref(mirror), Value::Int(public_only)] = *args else { unreachable!() };
    let class = vm.class_of_mirror(mirror)?;
    let methods: Vec<usize> = (0..class.file.methods.len())
        .filter(|&m| {
            let method = &class.file.methods[m];
            let public = method.access_flags.has(crate::flags::AccessFlags::PUBLIC);
            let wanted = match kind {
                Member::Method => !method.name.starts_with('<'),
                Member::Constructor => method.name == "<init>",
            };
            wanted && (public_only == 0 || public)
        })
        .collect();
    let element = FieldType::Object(kind.class().into());
    let array = vm.heap.new_array(&element, methods.len() as u32)?;
    vm.heap.roots.push(array);
    for (i, &m) in methods.iter().enumerate() {
        let member = vm.reflect(&class, m, kind)?;
        let array = *vm.heap.roots.last().expect("pushed above");
        let at = vm.heap.element(array, i as i32)?;
        vm.heap.write(at, &element, Value::Ref(member))?;
    }
    Ok(Some(Value::Ref(vm.heap.roots.pop().expect("pushed above"))))
}

impl Member {
    /// The class of the reflective object, and its constructor's descriptor:
    /// declaring class, name (for a method), parameter types, return type
    /// (for a method), exception types, modifiers, slot, generic signature
    /// and annotations.
    fn class(self) -> &'static str {
        match self {
            Member::Method => "java/lang/reflect/Method",
            Member::Constructor => "java/lang/reflect/Constructor",
        }
    }

    fn init(self) -> &'static str {
        match self {
            Member::Method => {
                "(Ljava/lang/Class;Ljava/lang/String;[Ljava/lang/Class;\
                 Ljava/lang/Class;[Ljava/lang/Class;IILjava/lang/String;[B[B[B)V"
            }
            Member::Constructor => {
                "(Ljava/lang/Class;[Ljava/lang/Class;[Ljava/lang/Class;\
                 IILjava/lang/String;[B[B)V"
            }
        }
    }
}
// @end

impl Vm {
    // @listing reflect-method
    /// A `Method` or `Constructor` for method `index` of `class`, made by its
    /// own constructor. Its arguments, the declaring class, the name, the
    /// parameter, return and exception types, are made one at a time and
    /// kept in the roots; the method's index is its `slot`.
    fn reflect(
        &mut self,
        class: &Rc<Class>,
        index: usize,
        kind: Member,
    ) -> Result<Ref> {
        let method = &class.file.methods[index];
        let (name, ty, flags) =
            (method.name.clone(), method.ty.clone(), method.access_flags);
        let exceptions = checked_exceptions(class, index)?;
        let base = self.heap.roots.len();
        let mirror = self.mirror(class)?;
        self.heap.roots.push(mirror);
        if kind == Member::Method {
            let name = self.intern(&name.encode_utf16().collect::<Vec<u16>>())?;
            self.heap.roots.push(name);
        }
        let parameters = self.class_array(&ty.params)?;
        self.heap.roots.push(parameters);
        if kind == Member::Method {
            let returns = match &ty.ret {
                Some(t) => self.component_mirror(&t.descriptor())?,
                None => self.primitive_mirror("void")?,
            };
            self.heap.roots.push(returns);
        }
        let exceptions = self.class_array(&exceptions)?;
        self.heap.roots.push(exceptions);
        let member_class = self.load(kind.class())?;
        self.initialize(&member_class)?; // as `new` would
        let object = self.heap.new_object(&member_class);
        let mut args = vec![Value::Ref(object)];
        args.extend(self.heap.roots.drain(base..).map(Value::Ref));
        let modifiers = i32::from(flags.0 & RECOGNIZED_METHOD_MODIFIERS);
        args.extend([Value::Int(modifiers), Value::Int(index as i32)]);
        let unused = if kind == Member::Method { 4 } else { 3 };
        args.extend(vec![Value::Ref(Ref::NULL); unused]); // signature and annotations
        let init = member_class.method("<init>", kind.init())?;
        self.heap.roots.push(object);
        let made = self.invoke_method(&member_class, init, &args);
        let object = self.heap.roots.pop().expect("pushed above");
        made.map(|_| object)
    }

    /// A `Class[]` of the mirrors of `types`.
    fn class_array(&mut self, types: &[FieldType]) -> Result<Ref> {
        let class_type = FieldType::Object("java/lang/Class".into());
        let array = self.heap.new_array(&class_type, types.len() as u32)?;
        self.heap.roots.push(array);
        for (i, ty) in types.iter().enumerate() {
            let mirror = self.component_mirror(&ty.descriptor())?;
            let array = *self.heap.roots.last().expect("pushed above");
            let at = self.heap.element(array, i as i32)?;
            self.heap.write(at, &class_type, Value::Ref(mirror))?;
        }
        Ok(self.heap.roots.pop().expect("pushed above"))
    }
    // @end
}

// @listing method-init
/// HotSpot's `JVM_RECOGNIZED_METHOD_MODIFIERS`: the access flags a
/// `Method` reports, which leave out `ACC_SUPER`'s bit and the rest.
const RECOGNIZED_METHOD_MODIFIERS: u16 = 0x1dff;

/// The classes in a method's `Exceptions` attribute, its `throws` clause.
fn checked_exceptions(class: &Class, index: usize) -> Result<Vec<FieldType>> {
    let method = &class.file.methods[index];
    let Some(a) = method.attributes.iter().find(|a| a.name == "Exceptions") else {
        return Ok(Vec::new());
    };
    let mut r = Reader::new(&a.info);
    let pool = &class.file.constant_pool;
    (0..r.u2()?)
        .map(|_| Ok(FieldType::Object(pool.class_name(r.u2()?)?.to_string())))
        .collect()
}
// @end

// @listing invoke0
/// `NativeAccessor.invoke0(Method, Object, Object[])`: calls the method a
/// `Method` stands for, with the arguments unboxed, and returns the result
/// boxed. An exception the method throws comes out wrapped in an
/// `InvocationTargetException`.
pub(crate) fn invoke0(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(method), Value::Ref(receiver), Value::Ref(arguments)] = *args
    else {
        unreachable!("checked descriptor")
    };
    let (class, slot) = vm.member_of(method, "java/lang/reflect/Method")?;
    let flags = class.file.methods[slot].access_flags;
    let ret = class.file.methods[slot].ty.ret.clone();
    let is_static = flags.has(crate::flags::AccessFlags::STATIC);
    let receiver = if is_static { None } else { Some(non_null(receiver)?) };
    let values = vm.arguments(&class, slot, receiver, arguments)?;
    let (target_class, target) = match receiver {
        Some(r) if !flags.has(crate::flags::AccessFlags::PRIVATE) => {
            let receiver = vm.class_of(r)?;
            select(&receiver, &class, slot).expect("a method to select")
        }
        _ => (class.clone(), slot),
    };
    if is_static {
        vm.initialize(&class)?;
    }
    match vm.invoke_method(&target_class, target, &values) {
        Ok(result) => Ok(Some(Value::Ref(vm.boxed(ret.as_ref(), result)?))),
        Err(Error::Thrown { exception, .. }) => Err(vm.invocation_target(exception)?),
        Err(other) => Err(other),
    }
}

impl Vm {
    /// The class and method index that a `Method` or `Constructor` stands
    /// for, from its `clazz` and `slot` fields.
    fn member_of(&mut self, member: Ref, kind: &str) -> Result<(Rc<Class>, usize)> {
        let class = self.load(kind)?;
        let read = |vm: &Vm, name: &str, ty: &str| -> Result<Value> {
            let slot = class.field(name, ty)?;
            Ok(vm.heap.read(member.0 + slot.offset, &slot.ty))
        };
        let (Value::Ref(mirror), Value::Int(slot)) =
            (read(self, "clazz", "Ljava/lang/Class;")?, read(self, "slot", "I")?)
        else {
            unreachable!("a member's fields")
        };
        Ok((self.class_of_mirror(mirror)?, slot as usize))
    }

    /// The values to call method `slot` of `class` with: the receiver, if
    /// any, then the elements of `arguments`, unboxed as the parameters'
    /// types require.
    fn arguments(
        &mut self,
        class: &Class,
        slot: usize,
        receiver: Option<Ref>,
        arguments: Ref,
    ) -> Result<Vec<Value>> {
        let params = class.file.methods[slot].ty.params.clone();
        let mut values: Vec<Value> = receiver.into_iter().map(Value::Ref).collect();
        let count = if arguments == Ref::NULL {
            0
        } else {
            self.heap.array_length(arguments)
        };
        if count as usize != params.len() {
            let m = format!(
                "wrong number of arguments: {count} expected: {}",
                params.len()
            );
            let class = "java/lang/IllegalArgumentException";
            return Err(Error::Exception { class, message: Some(m) });
        }
        let object = FieldType::Object("java/lang/Object".into());
        for (i, ty) in params.iter().enumerate() {
            let at = self.heap.element(arguments, i as i32)?;
            let Value::Ref(arg) = self.heap.read(at, &object) else {
                unreachable!("Object[]")
            };
            values.push(self.unbox(ty, arg)?);
        }
        Ok(values)
    }
}
// @end

impl Vm {
    // @listing boxing
    /// An argument for a parameter of type `ty`: a reference as it is, or a
    /// primitive from its wrapper object, which must be the right wrapper.
    fn unbox(&mut self, ty: &FieldType, arg: Ref) -> Result<Value> {
        let Some((wrapper_class, _)) = wrapper(ty) else {
            return Ok(Value::Ref(arg));
        };
        let mismatch = || Error::Exception {
            class: "java/lang/IllegalArgumentException",
            message: Some("argument type mismatch".into()),
        };
        if arg == Ref::NULL || self.class_of(arg)?.file.this_class != wrapper_class {
            return Err(mismatch());
        }
        let class = self.class_of(arg)?;
        let slot = class.field("value", &ty.descriptor())?;
        Ok(self.heap.read(arg.0 + slot.offset, &slot.ty))
    }

    /// A result as `Method.invoke` returns it: `null` for `void`, a primitive
    /// boxed by its wrapper's `valueOf`, a reference as it is.
    fn boxed(
        &mut self,
        ty: Option<&FieldType>,
        result: Option<Value>,
    ) -> Result<Ref> {
        let (Some(ty), Some(value)) = (ty, result) else { return Ok(Ref::NULL) };
        let Some((wrapper_class, _)) = wrapper(ty) else {
            let Value::Ref(r) = value else { unreachable!("a reference") };
            return Ok(r);
        };
        let class = self.load(wrapper_class)?;
        let descriptor = format!("({})L{wrapper_class};", ty.descriptor());
        match self.invoke(&class, "valueOf", &descriptor, &[value])? {
            Some(Value::Ref(r)) => Ok(r),
            _ => unreachable!("valueOf returns an object"),
        }
    }

    /// An `InvocationTargetException` whose target is `exception`.
    fn invocation_target(&mut self, exception: Ref) -> Result<Error> {
        self.heap.roots.push(exception);
        let wrapper =
            self.exception("java/lang/reflect/InvocationTargetException", None);
        let exception = self.heap.roots.pop().expect("pushed above");
        let (wrapper, class) =
            (wrapper?, self.load("java/lang/reflect/InvocationTargetException")?);
        let target = Value::Ref(exception);
        self.put(&class, wrapper, "target", "Ljava/lang/Throwable;", target)?;
        self.thrown(wrapper)
    }
    // @end
}

// @listing caller-class
/// `Reflection.getCallerClass()`: the class of the method that called the
/// caller-sensitive method asking, which is the top frame, since a native
/// has no frame of its own. Frames of reflection itself are skipped, so
/// that a method called through `Method.invoke` sees its real caller.
pub(crate) fn get_caller_class(vm: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    let caller = vm.frames.iter().rev().skip(1).find(|f| !is_reflection(f));
    let Some(class) = caller.map(|f| f.class.clone()) else {
        return Ok(Some(Value::Ref(Ref::NULL)));
    };
    Ok(Some(Value::Ref(vm.mirror(&class)?)))
}

/// Whether a frame belongs to reflection: `Method.invoke`, or the
/// accessors in `jdk.internal.reflect`.
fn is_reflection(frame: &Frame) -> bool {
    let class = &frame.class.file.this_class;
    (class == "java/lang/reflect/Method" && frame.name_is("invoke"))
        || class.starts_with("jdk/internal/reflect/")
}
// @end

// @listing constant-pool
/// `Class.getConstantPool()`: a `jdk.internal.reflect.ConstantPool`, which
/// reflection asks for before it parses annotations. The `Method`s carafe
/// makes have none, so the object is never used, and stays empty.
pub(crate) fn get_constant_pool(vm: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    let class = vm.load("jdk/internal/reflect/ConstantPool")?;
    vm.initialize(&class)?;
    Ok(Some(Value::Ref(vm.heap.new_object(&class))))
}
// @end

// @listing for-name
/// `Class.forName0(name, initialize, loader, caller)`: the class with this
/// binary name, such as `java.util.HashMap` or `[I`, loaded, and initialized
/// if asked; carafe has one class path, so the loader doesn't matter.
pub(crate) fn for_name(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(name), Value::Int(initialize), _, _] = *args else {
        unreachable!()
    };
    let name = vm.rust_string(non_null(name)?)?;
    let class = match vm.load(&name.replace('.', "/")) {
        Ok(class) => class,
        Err(Error::Exception { class: "java/lang/NoClassDefFoundError", .. }) => {
            let class = "java/lang/ClassNotFoundException";
            return Err(Error::Exception { class, message: Some(name) });
        }
        Err(other) => return Err(other),
    };
    if initialize != 0 {
        vm.initialize(&class)?;
    }
    Ok(Some(Value::Ref(vm.mirror(&class)?)))
}
// @end

// @listing new-instance
/// `NativeAccessor.newInstance0(Constructor, Object[])`: a new object of the
/// constructor's class, initialized first as `new` would, and constructed
/// with the arguments unboxed.
pub(crate) fn new_instance0(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(constructor), Value::Ref(arguments)] = *args else {
        unreachable!()
    };
    let (class, slot) = vm.member_of(constructor, "java/lang/reflect/Constructor")?;
    vm.heap.roots.push(arguments); // across initialization and allocation
    let initialized = vm.initialize(&class);
    let object = vm.heap.new_object(&class);
    let arguments = vm.heap.roots.pop().expect("pushed above");
    initialized?;
    vm.heap.roots.push(object);
    let values = vm.arguments(&class, slot, Some(object), arguments);
    let object = vm.heap.roots.pop().expect("pushed above");
    match vm.invoke_method(&class, slot, &values?) {
        Ok(_) => Ok(Some(Value::Ref(object))),
        Err(Error::Thrown { exception, .. }) => Err(vm.invocation_target(exception)?),
        Err(other) => Err(other),
    }
}
// @end

impl Vm {
    // @listing before-reflection
    /// Initializes `Method`, which HotSpot does at startup because the VM
    /// makes `Method` objects itself. Its superclass's initializer registers
    /// `JavaLangReflectAccess` with `SharedSecrets`, which `ReflectionFactory`
    /// reads once, when it is made. carafe waits until just before then.
    pub(crate) fn before_reflection(&mut self) -> Result<()> {
        let method = self.load("java/lang/reflect/Method")?;
        self.initialize(&method)
    }
    // @end
}

// @listing access-flags
/// `Reflection.getClassAccessFlags(Class)`: the class's access flags as its
/// class file has them, which for a nested class are not its modifiers.
pub(crate) fn get_class_access_flags(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let flags = match vm.class_of_mirror(mirror) {
        Ok(class) => i32::from(class.file.access_flags.0),
        Err(_) => 0x0411, // a primitive type: public final abstract
    };
    Ok(Some(Value::Int(flags)))
}
// @end

// @listing nesting
/// `Class.isHidden()`: true only for carafe's lambda classes, which HotSpot
/// would define as hidden classes.
pub(crate) fn is_hidden(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let hidden = vm
        .class_of_mirror(mirror)
        .is_ok_and(|c| c.file.this_class.contains("$$Lambda$"));
    Ok(Some(Value::Int(hidden.into())))
}

/// `Class.getDeclaringClass0()`: the class a member class is declared in,
/// from its `InnerClasses` entry, or `null`.
pub(crate) fn get_declaring_class(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let Ok(class) = vm.class_of_mirror(mirror) else {
        return Ok(Some(Value::Ref(Ref::NULL)));
    };
    let outer = match inner_entry(&class)? {
        Some((outer, _)) if outer != 0 => {
            let name = class.file.constant_pool.class_name(outer)?.to_string();
            let outer = vm.resolve_class(&class, &name)?;
            vm.mirror(&outer)?
        }
        _ => Ref::NULL,
    };
    Ok(Some(Value::Ref(outer)))
}

/// `Class.getSimpleBinaryName0()`: a nested class's name as its source
/// declared it, from its `InnerClasses` entry, or `null`.
pub(crate) fn get_simple_binary_name(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let Ok(class) = vm.class_of_mirror(mirror) else {
        return Ok(Some(Value::Ref(Ref::NULL)));
    };
    let name = match inner_entry(&class)? {
        Some((_, simple)) if simple != 0 => {
            let units: Vec<u16> =
                class.file.constant_pool.utf8(simple)?.encode_utf16().collect();
            vm.intern(&units)?
        }
        _ => Ref::NULL,
    };
    Ok(Some(Value::Ref(name)))
}

/// `Class.getEnclosingMethod0()`: for a local or anonymous class, from its
/// `EnclosingMethod` attribute, the enclosing class and the method's name
/// and descriptor, either `null` outside a method; `null` for other classes.
pub(crate) fn get_enclosing_method(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(mirror)] = *args else { unreachable!("checked descriptor") };
    let Ok(class) = vm.class_of_mirror(mirror) else {
        return Ok(Some(Value::Ref(Ref::NULL)));
    };
    let attributes = &class.file.attributes;
    let Some(a) = attributes.iter().find(|a| a.name == "EnclosingMethod") else {
        return Ok(Some(Value::Ref(Ref::NULL)));
    };
    let mut r = Reader::new(&a.info);
    let (outer, method) = (r.u2()?, r.u2()?);
    let pool = &class.file.constant_pool;
    let outer = pool.class_name(outer)?.to_string();
    let (name, descriptor) = match method {
        0 => (None, None),
        m => {
            let (name, descriptor) = pool.name_and_type(m)?;
            (Some(name.to_string()), Some(descriptor.to_string()))
        }
    };
    // Each element is kept in the roots until the array, made last, holds it.
    let base = vm.heap.roots.len();
    let outer = vm.resolve_class(&class, &outer)?;
    let outer = vm.mirror(&outer)?;
    vm.heap.roots.push(outer);
    for text in [name, descriptor] {
        let string = match text {
            Some(t) => vm.intern(&t.encode_utf16().collect::<Vec<u16>>())?,
            None => Ref::NULL,
        };
        vm.heap.roots.push(string);
    }
    let object = FieldType::Object("java/lang/Object".into());
    let array = vm.heap.new_array(&object, 3)?;
    for (i, &element) in vm.heap.roots[base..].to_vec().iter().enumerate() {
        let at = vm.heap.element(array, i as i32)?;
        vm.heap.write(at, &object, Value::Ref(element))?;
    }
    vm.heap.roots.truncate(base);
    Ok(Some(Value::Ref(array)))
}

/// The class's own entry in its `InnerClasses` attribute, if it is nested:
/// the indexes of its outer class and of its simple name, either 0 if the
/// source gave none.
fn inner_entry(class: &Class) -> Result<Option<(u16, u16)>> {
    let pool = &class.file.constant_pool;
    let Some(inner) = class.file.attributes.iter().find(|a| a.name == "InnerClasses")
    else {
        return Ok(None);
    };
    let mut r = Reader::new(&inner.info);
    for _ in 0..r.u2()? {
        let (inner, outer, simple, _flags) = (r.u2()?, r.u2()?, r.u2()?, r.u2()?);
        if pool.class_name(inner)? == class.file.this_class {
            return Ok(Some((outer, simple)));
        }
    }
    Ok(None)
}
// @end
