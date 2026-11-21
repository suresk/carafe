//! `LambdaMetafactory.metafactory`: a class for each lambda, written by carafe.

use crate::attribute::{Attribute, Code};
use crate::class::Class;
use crate::class_file::ClassFile;
use crate::descriptor::{FieldType, MethodDescriptor};
use crate::dynamic::CallSite;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::member::{Field, Method};
use crate::opcode::*;
use crate::pool::{Constant, ConstantPool};
use crate::resolve::nest_host;
use crate::value::Value;
use std::cell::Cell;
use std::rc::Rc;

// @listing handle
/// A `MethodHandle` constant with its names: the kind of reference (JVMS
/// 5.4.3.5), and the method, which may be an interface's.
struct Handle {
    kind: u8,
    class: String,
    name: String,
    descriptor: MethodDescriptor,
    interface: bool,
}

const REF_INVOKE_VIRTUAL: u8 = 5;
const REF_INVOKE_STATIC: u8 = 6;
const REF_INVOKE_SPECIAL: u8 = 7;
const REF_NEW_INVOKE_SPECIAL: u8 = 8;
const REF_INVOKE_INTERFACE: u8 = 9;

fn handle(pool: &ConstantPool, index: u16) -> Result<Handle> {
    let &Constant::MethodHandle { kind, reference } = pool.get(index)? else {
        return Err(Error::ClassFormat(format!("#{index} is not a MethodHandle")));
    };
    let interface =
        matches!(pool.get(reference)?, Constant::InterfaceMethodref { .. });
    let m = pool.member(reference)?;
    let descriptor = MethodDescriptor::parse(m.descriptor)?;
    Ok(Handle {
        kind,
        class: m.class.into(),
        name: m.name.into(),
        descriptor,
        interface,
    })
}

/// The `MethodType` constant at `index`, parsed.
fn method_type(pool: &ConstantPool, index: u16) -> Result<MethodDescriptor> {
    let &Constant::MethodType { descriptor } = pool.get(index)? else {
        return Err(Error::ClassFormat(format!("#{index} is not a MethodType")));
    };
    MethodDescriptor::parse(pool.utf8(descriptor)?)
}
// @end

// @listing alternatives
/// `altMetafactory`'s further static arguments: an `int` of flags, then, if
/// flag 2 is set, a count and that many marker interfaces, and, if flag 4
/// is set, a count and that many method types for bridge methods. Flag 1
/// makes the lambda `Serializable`. Returns the interfaces the class
/// implements and the types of its bridge methods.
fn alternatives(
    pool: &ConstantPool,
    interface: &str,
    extra: &[u16],
) -> Result<(Vec<String>, Vec<MethodDescriptor>)> {
    let mut interfaces = vec![interface.to_string()];
    let mut bridges = Vec::new();
    let int = |index: Option<&u16>| match index.map(|&i| pool.get(i)) {
        Some(Ok(&Constant::Integer(v))) if v >= 0 => Ok(v as usize),
        _ => Err(Error::ClassFormat("altMetafactory expects an int >= 0".into())),
    };
    let Some((flags, mut rest)) = extra.split_first() else {
        return Ok((interfaces, bridges));
    };
    let flags = int(Some(flags))?;
    if flags & 1 != 0 {
        interfaces.push("java/io/Serializable".into());
    }
    for flag in [2, 4] {
        if flags & flag == 0 {
            continue;
        }
        let count = int(rest.first())?;
        let Some(items) = rest.get(1..1 + count) else {
            return Err(Error::ClassFormat(
                "altMetafactory runs out of arguments".into(),
            ));
        };
        for &item in items {
            match flag {
                2 => interfaces.push(pool.class_name(item)?.to_string()),
                _ => bridges.push(method_type(pool, item)?),
            }
        }
        rest = &rest[1 + count..];
    }
    Ok((interfaces, bridges))
}
// @end

impl Vm {
    // @listing lambda-site
    /// Links a call site of `LambdaMetafactory.metafactory`. The call site's
    /// descriptor takes the captured values and returns the functional
    /// interface; the static arguments are the interface method's erased type,
    /// the method that implements it and its type in this use.
    /// `altMetafactory` adds flags, more interfaces and bridge methods.
    pub(crate) fn lambda_site(
        &mut self,
        caller: &Class,
        name: &str,
        factory: MethodDescriptor,
        statics: &[u16],
    ) -> Result<CallSite> {
        let pool = &caller.file.constant_pool;
        let (
            &[erased, target, instantiated, ref extra @ ..],
            Some(FieldType::Object(interface)),
        ) = (statics, &factory.ret)
        else {
            let m = "a metafactory call site with the wrong arguments".to_string();
            return Err(linkage("java/lang/BootstrapMethodError", m));
        };
        let erased = method_type(pool, erased)?;
        let target = handle(pool, target)?;
        let instantiated = method_type(pool, instantiated)?;
        let (interfaces, bridges) = alternatives(pool, interface, extra)?;
        let class_name =
            format!("{}$$Lambda${}", caller.file.this_class, self.lambdas);
        self.lambdas += 1;
        let host = nest_host(caller);
        let types = Types { erased, instantiated, captures: factory.params.clone() };
        let shape = Shape {
            name: &class_name,
            caller: &caller.file.this_class,
            host: &host,
            interfaces: &interfaces,
            method: name,
        };
        let file = lambda_class(&shape, &types, &bridges, &target)?;
        let object = self.load("java/lang/Object")?;
        let interfaces =
            interfaces.iter().map(|i| self.load(i)).collect::<Result<_>>()?;
        let mut class = Class::extending(file, object);
        class.interfaces = interfaces;
        let class = Rc::new(class);
        self.classes.insert(class_name, class.clone());
        let instance = Cell::new(Ref::NULL);
        Ok(CallSite::Lambda { class, captures: factory.params, instance })
    }
    // @end

    // @listing new-lambda
    /// Runs a lambda's call site: a new object of its class, with the captured
    /// values popped into its fields. A lambda that captures nothing has one
    /// object, made the first time, as in HotSpot.
    pub(crate) fn new_lambda(
        &mut self,
        class: &Rc<Class>,
        captures: &[FieldType],
        instance: &Cell<Ref>,
    ) -> Result<()> {
        if instance.get() == Ref::NULL {
            // The captured values stay on the operand stack, as roots, until
            // the object is allocated.
            let object = self.heap.new_object(class);
            let frame = self.frames.last_mut().expect("a frame");
            for (i, ty) in captures.iter().enumerate().rev() {
                let value = frame.pop()?;
                let slot = class.field(&format!("arg${i}"), &ty.descriptor())?;
                self.heap.write(object.0 + slot.offset, &slot.ty, value)?;
            }
            if !captures.is_empty() {
                return self
                    .frames
                    .last_mut()
                    .expect("a frame")
                    .push(Value::Ref(object));
            }
            instance.set(object);
        }
        self.frames.last_mut().expect("a frame").push(Value::Ref(instance.get()))
    }
    // @end
}

// @listing types
/// The types a lambda class adapts between: the interface method's, erased
/// to what the interface declares, and as this use of it instantiates it,
/// and the types of the captured values.
#[derive(Clone)]
struct Types {
    erased: MethodDescriptor,
    instantiated: MethodDescriptor,
    captures: Vec<FieldType>,
}
// @end

// @listing lambda-class
/// The names a lambda class is made with: its own, its caller's, its nest
/// host's, the interfaces it implements and the interface method's.
struct Shape<'a> {
    name: &'a str,
    caller: &'a str,
    host: &'a str,
    interfaces: &'a [String],
    method: &'a str,
}

/// The class file of a lambda class: `final`, extending `Object`, with a
/// field `arg$0`, `arg$1` and so on for each captured value, and the
/// interface method, and any bridges, calling `target`. Its `NestHost` is
/// the caller's, so that it may call the caller's private methods.
fn lambda_class(
    shape: &Shape,
    types: &Types,
    bridges: &[MethodDescriptor],
    target: &Handle,
) -> Result<ClassFile> {
    let mut a = Assembler::default();
    let mut methods = Vec::new();
    for erased in std::iter::once(&types.erased).chain(bridges) {
        let types = Types { erased: erased.clone(), ..types.clone() };
        methods.push(Method {
            access_flags: AccessFlags(AccessFlags::PUBLIC),
            name: shape.method.to_string(),
            descriptor: descriptor(erased),
            ty: erased.clone(),
            code: Some(a.body(shape, &types, target)?),
            attributes: Vec::new(),
        });
    }
    let fields = types.captures.iter().enumerate().map(|(i, ty)| Field {
        access_flags: AccessFlags(AccessFlags::PRIVATE | AccessFlags::FINAL),
        name: format!("arg${i}"),
        descriptor: ty.descriptor(),
        ty: ty.clone(),
        constant_value: None,
        attributes: Vec::new(),
    });
    let host = a.class(shape.host).to_be_bytes().into();
    let flags = AccessFlags::FINAL | AccessFlags::SYNTHETIC | AccessFlags::SUPER;
    Ok(ClassFile {
        minor_version: 0,
        major_version: crate::class_file::MAX_MAJOR,
        constant_pool: ConstantPool::new(a.pool),
        access_flags: AccessFlags(flags),
        this_class: shape.name.to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: shape.interfaces.to_vec(),
        fields: fields.collect(),
        methods,
        attributes: vec![Attribute { name: "NestHost".into(), info: host }],
    })
}

/// A method descriptor as text: `(ILjava/lang/String;)V`.
fn descriptor(m: &MethodDescriptor) -> String {
    let params: String = m.params.iter().map(|p| p.descriptor()).collect();
    let ret = m.ret.as_ref().map_or("V".to_string(), |r| r.descriptor());
    format!("({params}){ret}")
}
// @end

// @listing assembler
/// Bytecode and the constant pool it refers to, built together.
struct Assembler {
    pool: Vec<Constant>,
    code: Vec<u8>,
}

impl Default for Assembler {
    fn default() -> Assembler {
        Assembler { pool: vec![Constant::Unusable], code: Vec::new() }
    }
}

impl Assembler {
    /// The index of `constant` in the pool, adding it if it isn't there.
    fn constant(&mut self, constant: Constant) -> u16 {
        let found = self.pool.iter().position(|c| *c == constant);
        found.unwrap_or_else(|| {
            self.pool.push(constant);
            self.pool.len() - 1
        }) as u16
    }

    fn utf8(&mut self, text: &str) -> u16 {
        self.constant(Constant::Utf8(text.to_string()))
    }

    fn class(&mut self, name: &str) -> u16 {
        let name = self.utf8(name);
        self.constant(Constant::Class { name })
    }

    /// A `Fieldref`, `Methodref` or `InterfaceMethodref`, which `make` builds.
    fn member(
        &mut self,
        make: fn(u16, u16) -> Constant,
        class: &str,
        name: &str,
        descriptor: &str,
    ) -> u16 {
        let class = self.class(class);
        let (name, descriptor) = (self.utf8(name), self.utf8(descriptor));
        let name_and_type = self.constant(Constant::NameAndType { name, descriptor });
        self.constant(make(class, name_and_type))
    }

    /// An instruction with no operands, or with a 1- or 2-byte operand.
    fn op(&mut self, opcode: u8) {
        self.code.push(opcode);
    }

    fn op_u1(&mut self, opcode: u8, operand: u8) {
        self.code.extend([opcode, operand]);
    }

    fn op_u2(&mut self, opcode: u8, operand: u16) {
        self.code.push(opcode);
        self.code.extend(operand.to_be_bytes());
    }
}

fn field_ref(class: u16, name_and_type: u16) -> Constant {
    Constant::Fieldref { class, name_and_type }
}

fn method_ref(class: u16, name_and_type: u16) -> Constant {
    Constant::Methodref { class, name_and_type }
}

fn interface_ref(class: u16, name_and_type: u16) -> Constant {
    Constant::InterfaceMethodref { class, name_and_type }
}
// @end

impl Assembler {
    // @listing body
    /// The interface method's code: the captured values from the fields,
    /// then the method's own arguments, each adapted to the type `target`
    /// takes; the call; and the result, adapted to what the method returns.
    fn body(
        &mut self,
        shape: &Shape,
        types: &Types,
        target: &Handle,
    ) -> Result<Code> {
        let receiver = matches!(
            target.kind,
            REF_INVOKE_VIRTUAL | REF_INVOKE_SPECIAL | REF_INVOKE_INTERFACE
        );
        let receiver = receiver.then(|| FieldType::Object(target.class.clone()));
        let params: Vec<FieldType> = receiver
            .into_iter()
            .chain(target.descriptor.params.iter().cloned())
            .collect();
        if types.captures.len() + types.erased.params.len() != params.len() {
            let m = format!("{} can't implement the interface method", target.name);
            return Err(linkage("java/lang/invoke/LambdaConversionException", m));
        }
        if target.kind == REF_NEW_INVOKE_SPECIAL {
            let class = self.class(&target.class);
            self.op_u2(NEW, class);
            self.op(DUP);
        }
        for (i, ty) in types.captures.iter().enumerate() {
            self.op(ALOAD_0);
            let field = self.member(
                field_ref,
                shape.name,
                &format!("arg${i}"),
                &ty.descriptor(),
            );
            self.op_u2(GETFIELD, field);
            self.convert(ty, ty, &params[i])?;
        }
        let mut slot = 1;
        for (j, ty) in types.erased.params.iter().enumerate() {
            let load = [ILOAD, LLOAD, FLOAD, DLOAD, ALOAD][kind(ty)];
            self.op_u1(load, u8::try_from(slot).map_err(|_| too_many())?);
            slot += ty.slots();
            let to = &params[types.captures.len() + j];
            self.convert(ty, &types.instantiated.params[j], to)?;
        }
        let returns = self.call(shape.caller, target)?;
        match (&returns, &types.erased.ret) {
            (None, None) => self.op(RETURN),
            (Some(r), None) => {
                self.op(if r.slots() == 2 { POP2 } else { POP });
                self.op(RETURN);
            }
            (Some(r), Some(t)) => {
                self.convert(r, r, t)?;
                self.op([IRETURN, LRETURN, FRETURN, DRETURN, ARETURN][kind(t)]);
            }
            (None, Some(_)) => {
                let m = format!("{} returns void", target.name);
                return Err(linkage("java/lang/invoke/LambdaConversionException", m));
            }
        }
        let arguments: usize = params.iter().map(FieldType::slots).sum();
        Ok(Code {
            max_stack: u16::try_from(arguments + 4).map_err(|_| too_many())?,
            max_locals: u16::try_from(slot).map_err(|_| too_many())?,
            bytecode: std::mem::take(&mut self.code),
            handlers: Vec::new(),
            attributes: Vec::new(),
        })
    }

    /// The call to `target`, made from a lambda class of `caller`, and the
    /// type it leaves on the stack. An `invokeSpecial` handle, which only
    /// `javac` before Java 11 emits, for a private method of the caller, is
    /// called as `invokevirtual` or `invokeinterface`, as `LambdaMetafactory`
    /// calls it, since `invokespecial` can't name another class's method.
    fn call(&mut self, caller: &str, target: &Handle) -> Result<Option<FieldType>> {
        let (class, name) = (&target.class, &target.name);
        let text = descriptor(&target.descriptor);
        let make = if target.interface { interface_ref } else { method_ref };
        let method = self.member(make, class, name, &text);
        match target.kind {
            REF_INVOKE_STATIC => self.op_u2(INVOKESTATIC, method),
            REF_INVOKE_SPECIAL if class != caller => {
                let m = format!("a REF_invokeSpecial handle to {class}.{name}");
                return Err(Error::Unsupported(m));
            }
            REF_INVOKE_VIRTUAL | REF_INVOKE_SPECIAL if !target.interface => {
                self.op_u2(INVOKEVIRTUAL, method)
            }
            REF_NEW_INVOKE_SPECIAL => self.op_u2(INVOKESPECIAL, method),
            REF_INVOKE_INTERFACE | REF_INVOKE_SPECIAL => {
                self.op_u2(INVOKEINTERFACE, method);
                let count = 1 + target.descriptor.param_slots();
                self.code.extend([u8::try_from(count).map_err(|_| too_many())?, 0]);
            }
            kind => {
                return Err(Error::Unsupported(format!(
                    "a method handle of kind {kind}"
                )));
            }
        }
        Ok(match target.kind {
            REF_NEW_INVOKE_SPECIAL => Some(FieldType::Object(class.clone())),
            _ => target.descriptor.ret.clone(),
        })
    }
    // @end

    // @listing convert
    /// Converts the value on top of the stack, of type `from`, to type `to`:
    /// widening a primitive, boxing it, unboxing a wrapper, or casting a
    /// reference. `hint` is the precise type of the value, which says which
    /// wrapper an `Object` holds when it is unboxed.
    fn convert(
        &mut self,
        from: &FieldType,
        hint: &FieldType,
        to: &FieldType,
    ) -> Result<()> {
        match (wrapper(from), wrapper(to)) {
            (Some(_), Some(_)) => self.widen(from, to),
            (Some((box_class, _)), None) => {
                let boxing = format!("({})L{box_class};", from.descriptor());
                let method = self.member(method_ref, box_class, "valueOf", &boxing);
                self.op_u2(INVOKESTATIC, method);
                Ok(())
            }
            (None, Some((to_box, to_value))) => {
                let unboxed = match hint {
                    FieldType::Object(name) => primitive_of(name),
                    _ => None,
                };
                // No wrapper named: the base wrapper, `Number` for a number.
                let (box_class, value, unboxed) = match unboxed {
                    Some(unboxed) => {
                        let (box_class, value) =
                            wrapper(&unboxed).expect("a primitive");
                        (box_class, value, unboxed)
                    }
                    None if matches!(to, FieldType::Boolean | FieldType::Char) => {
                        (to_box, to_value, to.clone())
                    }
                    None => ("java/lang/Number", to_value, to.clone()),
                };
                let class = self.class(box_class);
                self.op_u2(CHECKCAST, class);
                let getter = format!("(){}", unboxed.descriptor());
                let method = self.member(method_ref, box_class, value, &getter);
                self.op_u2(INVOKEVIRTUAL, method);
                self.widen(&unboxed, to)
            }
            (None, None) => {
                if to != from && *to != FieldType::Object("java/lang/Object".into()) {
                    let class = match to {
                        FieldType::Object(name) => self.class(name),
                        array => self.class(&array.descriptor()),
                    };
                    self.op_u2(CHECKCAST, class);
                }
                Ok(())
            }
        }
    }

    /// A widening primitive conversion (JLS 5.1.2), such as `i2l`. Kind 0
    /// is `boolean`, `byte`, `char`, `short` and `int` alike; carafe trusts
    /// `javac` not to ask for a narrowing among them, which
    /// `LambdaMetafactory` would reject.
    fn widen(&mut self, from: &FieldType, to: &FieldType) -> Result<()> {
        let op = match (kind(from), kind(to)) {
            (a, b) if a == b => return Ok(()),
            (0, 1) => I2L,
            (0, 2) => I2F,
            (0, 3) => I2D,
            (1, 2) => L2F,
            (1, 3) => L2D,
            (2, 3) => F2D,
            _ => {
                return Err(Error::Unsupported(format!("converting {from} to {to}")));
            }
        };
        self.op(op);
        Ok(())
    }
    // @end
}

// @listing wrappers
/// For a primitive type, its wrapper class and the method that unboxes it.
pub(crate) fn wrapper(ty: &FieldType) -> Option<(&'static str, &'static str)> {
    Some(match ty {
        FieldType::Boolean => ("java/lang/Boolean", "booleanValue"),
        FieldType::Byte => ("java/lang/Byte", "byteValue"),
        FieldType::Char => ("java/lang/Character", "charValue"),
        FieldType::Short => ("java/lang/Short", "shortValue"),
        FieldType::Int => ("java/lang/Integer", "intValue"),
        FieldType::Long => ("java/lang/Long", "longValue"),
        FieldType::Float => ("java/lang/Float", "floatValue"),
        FieldType::Double => ("java/lang/Double", "doubleValue"),
        FieldType::Object(_) | FieldType::Array(_) => return None,
    })
}

/// The primitive type a wrapper class holds.
fn primitive_of(class: &str) -> Option<FieldType> {
    "ZBCSIJFD"
        .chars()
        .map(|c| FieldType::parse(&c.to_string()).expect("a primitive"))
        .find(|p| wrapper(p).is_some_and(|(name, _)| name == class))
}

/// How a value of this type is loaded and returned: 0 as an `int`, then
/// `long`, `float`, `double` and reference, the order of the instructions.
fn kind(ty: &FieldType) -> usize {
    match ty {
        FieldType::Long => 1,
        FieldType::Float => 2,
        FieldType::Double => 3,
        FieldType::Object(_) | FieldType::Array(_) => 4,
        _ => 0,
    }
}

fn too_many() -> Error {
    Error::Unsupported("a lambda with more than 255 slots of arguments".into())
}
// @end
