//! The interpreter: a loop that executes one instruction at a time.

use crate::bytecode::{Instruction, Operand};
use crate::casts::is_subtype;
use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::exceptions::TraceFrame;
use crate::flags::AccessFlags;
use crate::frame::Frame;
use crate::heap::{Heap, Ref};
use crate::loader::{ClassPath, linkage};
use crate::mirror::is_class;
use crate::numeric::numeric;
use crate::opcode::*;
use crate::pool::Constant;
use crate::resolve::{
    dotted, external, find_in_superclasses, is_subclass, maximally_specific, package,
};
use crate::strings::is_string;
use crate::value::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

// @listing vm
/// The virtual machine: the stack of frames of the method calls in
/// progress, and a count of the instructions executed so far.
#[derive(Default)]
pub struct Vm {
    pub(crate) frames: Vec<Frame>,
    pub executed: u64,
    // @hide
    /// The number of calls made by `invoke` instructions.
    pub calls: u64,
    /// The most frames that were ever on the stack at once.
    pub deepest: usize,
    /// Every object the program has created.
    pub heap: Heap,
    /// Where `load` finds classes.
    pub class_path: ClassPath,
    /// The classes loaded so far, by name.
    pub(crate) classes: HashMap<String, Rc<Class>>,
    /// The names of the classes loaded so far, in the order loading ended.
    pub loaded: Vec<String>,
    /// The classes whose loading has started and not ended.
    pub(crate) loading: Vec<String>,
    /// The interned strings, by their UTF-16 code units.
    pub(crate) strings: HashMap<Vec<u16>, Ref>,
    /// What the program has written to file descriptors 1 and 2.
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// The native methods called so far, as `class.name` + descriptor.
    pub natives: BTreeSet<String>,
    /// How many directories at the start of the class path are the boot
    /// class path, which `Vm::boot` sets.
    pub boot_dirs: usize,
    /// The `Class` objects of the primitive types, by name: `int`, `void`.
    pub(crate) primitive_mirrors: BTreeMap<String, Ref>,
    /// The class each mirror stands for, by the mirror's reference.
    pub(crate) mirror_classes: HashMap<Ref, Rc<Class>>,
    /// Extra frames allowed while the VM constructs an exception, so that a
    /// `StackOverflowError` can be created at the limit.
    pub(crate) headroom: usize,
    /// Each exception's stack trace, recorded by `fillInStackTrace`.
    pub(crate) backtraces: HashMap<Ref, Vec<TraceFrame>>,
    /// How many lambda classes carafe has made, which numbers the next one.
    pub(crate) lambdas: usize,
    /// The main thread's `Thread` object, once something has asked for it.
    pub thread: Option<Ref>,
    /// What the program reads from standard input, and how much it has read.
    pub stdin: Vec<u8>,
    pub(crate) stdin_read: usize,
    // @show
}

/// What happens after an instruction.
enum Flow {
    /// Go on to the next instruction.
    Next,
    /// Go on at this pc.
    Jump(u16),
    /// Return from the method, with this result.
    Return(Option<Value>),
}

impl Vm {
    pub fn new() -> Vm {
        Vm::default()
    }

    /// Calls a static method of `class` from Rust and runs it to completion.
    pub fn invoke(
        &mut self,
        class: &Rc<Class>,
        name: &str,
        descriptor: &str,
        args: &[Value],
    ) -> Result<Option<Value>> {
        let method = class.method(name, descriptor)?;
        if !class.file.methods[method].access_flags.has(AccessFlags::STATIC) {
            return Err(Error::Unsupported(format!(
                "calling {name}, which isn't static"
            )));
        }
        // @hide
        let name = class.file.this_class.clone();
        self.classes.entry(name).or_insert_with(|| class.clone());
        self.initialize(class)?;
        // @show
        let depth = self.frames.len();
        self.frames.push(Frame::new(class.clone(), method, args)?);
        let result = self.run(depth);
        self.frames.truncate(depth);
        result
    }
}
// @end

impl Vm {
    // @listing run
    /// Executes instructions until the frame at `depth` returns. An error is
    /// caught by a handler in one of the frames, or leaves `run` as `Thrown`.
    pub(crate) fn run(&mut self, depth: usize) -> Result<Option<Value>> {
        loop {
            let frame = self.frames.last_mut().expect("run has a frame");
            let code = frame.code.clone();
            let Some(instruction) = code.get(frame.next) else {
                return Err(Error::Verify(format!(
                    "{} runs off its end",
                    frame.name()
                )));
            };
            frame.next += 1;
            self.executed += 1;
            let flow = match self.execute(instruction) {
                Ok(flow) => flow,
                Err(error) => {
                    self.catch(error, depth)?;
                    continue;
                }
            };
            match flow {
                Flow::Next => {}
                Flow::Jump(pc) => {
                    let frame = self.frames.last_mut().expect("run has a frame");
                    // decode() checked that every target starts an instruction.
                    frame.next =
                        code.binary_search_by_key(&pc, |i| i.pc).expect("a target");
                }
                Flow::Return(value) => {
                    self.frames.pop();
                    if self.frames.len() == depth {
                        return Ok(value);
                    }
                    if let Some(value) = value {
                        self.frames.last_mut().expect("a caller").push(value)?;
                    }
                }
            }
        }
    }
    // @end

    // @listing execute
    /// Executes one instruction in the current frame.
    fn execute(&mut self, i: &Instruction) -> Result<Flow> {
        let frame = self.frames.last_mut().expect("execute has a frame");
        match (i.opcode, &i.operand) {
            (NOP, _) => {}
            (ICONST_M1..=ICONST_5, _) => {
                frame.push(Value::Int(i32::from(i.opcode) - i32::from(ICONST_0)))?
            }
            (BIPUSH | SIPUSH, &Operand::Value(v)) => {
                frame.push(Value::Int(v.into()))?
            }
            // @hide
            (LDC | LDC_W, &Operand::Constant(index))
                if is_class(&frame.class, index) =>
            {
                let mirror = self.ldc_class(index)?;
                self.frames.last_mut().expect("a frame").push(Value::Ref(mirror))?;
            }
            (LDC | LDC_W, &Operand::Constant(index))
                if is_string(&frame.class, index) =>
            {
                let string = self.ldc_string(index)?;
                self.frames.last_mut().expect("a frame").push(Value::Ref(string))?;
            }
            // @show
            (LDC | LDC_W | LDC2_W, &Operand::Constant(index)) => {
                let value = constant(&frame.class, index)?;
                frame.push(value)?;
            }
            (ILOAD..=ALOAD, &Operand::Local(n)) => frame.push(frame.load(n)?)?,
            (ILOAD_0..=ALOAD_3, _) => {
                let n = (i.opcode - ILOAD_0) % 4;
                frame.push(frame.load(n.into())?)?;
            }
            (ISTORE..=ASTORE, &Operand::Local(n)) => {
                let value = frame.pop()?;
                frame.store(n, value)?;
            }
            (ISTORE_0..=ASTORE_3, _) => {
                let value = frame.pop()?;
                frame.store(((i.opcode - ISTORE_0) % 4).into(), value)?;
            }
            (IINC, &Operand::Iinc { local, delta }) => {
                let Value::Int(v) = frame.load(local)? else {
                    return Err(Error::Verify(format!(
                        "iinc of a non-int at {}",
                        i.pc
                    )));
                };
                frame.store(local, Value::Int(v.wrapping_add(delta.into())))?;
            }
            (POP..=SWAP, _) => shuffle(frame, i.opcode)?,
            (IADD | ISUB | IMUL | IDIV | IREM, _)
            | (ISHL | ISHR | IUSHR | IAND | IOR | IXOR, _) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Value::Int(int_op(i.opcode, a, b)?))?;
            }
            (INEG, _) => {
                let a = frame.pop_int()?;
                frame.push(Value::Int(a.wrapping_neg()))?;
            }
            // @hide
            (INVOKEVIRTUAL, &Operand::Constant(index)) => {
                return self.invoke_virtual(index);
            }
            (LCONST_0..=DCONST_1 | LADD..=LXOR | I2L..=DCMPG, _) => {
                numeric(frame, i.opcode)?
            }
            (ACONST_NULL, _) => frame.push(Value::Ref(Ref::NULL))?,
            (NEWARRAY | ANEWARRAY | MULTIANEWARRAY | ARRAYLENGTH, _)
            | (IALOAD..=SALOAD | IASTORE..=SASTORE, _) => self.array_op(i)?,
            (INVOKEDYNAMIC, &Operand::Dynamic(index)) => {
                self.invoke_dynamic(index, i.pc)?
            }
            (ATHROW, _) => {
                let exception = non_null(frame.pop_ref()?)?;
                return Err(self.thrown(exception)?);
            }
            (MONITORENTER | MONITOREXIT, _) => {
                // One thread: every monitor is free to enter (JVM4 adds threads).
                non_null(frame.pop_ref()?)?;
            }
            (CHECKCAST | INSTANCEOF, &Operand::Constant(index)) => {
                self.type_check(i.opcode, index)?
            }
            (INVOKEINTERFACE, &Operand::Interface { index, .. }) => {
                return self.invoke_interface(index);
            }
            (GETSTATIC | PUTSTATIC, &Operand::Constant(index)) => {
                self.static_op(i.opcode, index)?
            }
            (NEW | GETFIELD | PUTFIELD, &Operand::Constant(index)) => {
                self.object_op(i.opcode, index)?
            }
            (INVOKESPECIAL, &Operand::Constant(index)) => {
                return self.invoke_special(index);
            }
            (INVOKESTATIC, &Operand::Constant(index)) => {
                return self.invoke_static(index);
            }
            // @show
            _ => return branch(frame, i),
        }
        Ok(Flow::Next)
    }
    // @end
}

// @listing branch
/// The instructions that may change where execution goes next.
fn branch(frame: &mut Frame, i: &Instruction) -> Result<Flow> {
    let taken = |yes: bool, to: u16| if yes { Flow::Jump(to) } else { Flow::Next };
    Ok(match (i.opcode, &i.operand) {
        (IFEQ..=IFLE, &Operand::Target(to)) => {
            let a = frame.pop_int()?;
            taken(holds(i.opcode - IFEQ, a, 0), to)
        }
        (IF_ICMPEQ..=IF_ICMPLE, &Operand::Target(to)) => {
            let b = frame.pop_int()?;
            let a = frame.pop_int()?;
            taken(holds(i.opcode - IF_ICMPEQ, a, b), to)
        }
        (GOTO | GOTO_W, &Operand::Target(to)) => Flow::Jump(to),
        (TABLESWITCH, Operand::TableSwitch { default, low, targets }) => {
            let key = i64::from(frame.pop_int()?) - i64::from(*low);
            let target = usize::try_from(key).ok().and_then(|k| targets.get(k));
            Flow::Jump(*target.unwrap_or(default))
        }
        (LOOKUPSWITCH, Operand::LookupSwitch { default, pairs }) => {
            let key = frame.pop_int()?;
            match pairs.binary_search_by_key(&key, |&(k, _)| k) {
                Ok(at) => Flow::Jump(pairs[at].1),
                Err(_) => Flow::Jump(*default),
            }
        }
        // @hide
        (IRETURN, _) => Flow::Return(Some(narrow_return(frame)?)),
        (IFNULL | IFNONNULL, &Operand::Target(to)) => {
            let is_null = frame.pop_ref()? == Ref::NULL;
            taken(is_null == (i.opcode == IFNULL), to)
        }
        (IF_ACMPEQ | IF_ACMPNE, &Operand::Target(to)) => {
            let b = frame.pop_ref()?;
            let a = frame.pop_ref()?;
            taken((a == b) == (i.opcode == IF_ACMPEQ), to)
        }
        // @show
        (IRETURN..=ARETURN, _) => Flow::Return(Some(frame.pop()?)),
        (RETURN, _) => Flow::Return(None),
        _ => {
            let what = format!("{} at {} in {}", i.mnemonic(), i.pc, frame.name());
            return Err(Error::Unsupported(what));
        }
    })
}

/// Whether condition `c` holds, numbered as the JVMS orders the branches:
/// 0 `eq`, 1 `ne`, 2 `lt`, 3 `ge`, 4 `gt`, 5 `le`.
fn holds(c: u8, a: i32, b: i32) -> bool {
    match c {
        0 => a == b,
        1 => a != b,
        2 => a < b,
        3 => a >= b,
        4 => a > b,
        _ => a <= b,
    }
}
// @end

// @listing int-op
/// The `int` arithmetic of JVMS 6.5: two's complement, wrapping on overflow,
/// with shift distances taken modulo 32.
fn int_op(opcode: u8, a: i32, b: i32) -> Result<i32> {
    Ok(match opcode {
        IADD => a.wrapping_add(b),
        ISUB => a.wrapping_sub(b),
        IMUL => a.wrapping_mul(b),
        IDIV | IREM if b == 0 => return Err(divide_by_zero()),
        IDIV => a.wrapping_div(b),
        IREM => a.wrapping_rem(b),
        ISHL => a.wrapping_shl(b as u32),
        ISHR => a.wrapping_shr(b as u32),
        IUSHR => (a as u32).wrapping_shr(b as u32) as i32,
        IAND => a & b,
        IOR => a | b,
        _ => a ^ b,
    })
}

fn divide_by_zero() -> Error {
    let message = Some("/ by zero".to_string());
    Error::Exception { class: "java/lang/ArithmeticException", message }
}
// @end

// @listing shuffle
/// The stack instructions, `pop` to `swap`. Each takes some slots off the
/// stack and pushes copies of them back in a new order. `0` is the slot
/// that was on top.
fn shuffle(frame: &mut Frame, opcode: u8) -> Result<()> {
    let (take, order): (usize, &[usize]) = match opcode {
        POP => (1, &[]),
        POP2 => (2, &[]),
        DUP => (1, &[0, 0]),
        DUP_X1 => (2, &[0, 1, 0]),
        DUP_X2 => (3, &[0, 2, 1, 0]),
        DUP2 => (2, &[1, 0, 1, 0]),
        DUP2_X1 => (3, &[1, 0, 2, 1, 0]),
        DUP2_X2 => (4, &[1, 0, 3, 2, 1, 0]),
        _ => (2, &[0, 1]), // SWAP
    };
    let mut slots = Vec::with_capacity(take);
    for _ in 0..take {
        slots.push(frame.pop_slot()?);
    }
    for &slot in order {
        frame.push_slot(slots[slot])?;
    }
    Ok(())
}
// @end

// @listing constant
/// The value of a numeric constant, for `ldc`, `ldc_w` and `ldc2_w`.
fn constant(class: &Class, index: u16) -> Result<Value> {
    Ok(match class.file.constant_pool.get(index)? {
        Constant::Integer(v) => Value::Int(*v),
        Constant::Float(v) => Value::Float(*v),
        Constant::Long(v) => Value::Long(*v),
        Constant::Double(v) => Value::Double(*v),
        other => return Err(Error::Unsupported(format!("ldc of {other:?}"))),
    })
}
// @end

// @listing max-frames
/// The most frames carafe allows on the stack before it throws
/// `StackOverflowError`.
pub const MAX_FRAMES: usize = 10_000;
// @end

impl Vm {
    // @listing invoke-static
    /// `invokestatic`: resolves the method, pops the arguments from the
    /// caller's operand stack and pushes a frame for the method, which runs next.
    fn invoke_static(&mut self, index: u16) -> Result<Flow> {
        let from =
            self.frames.last().expect("invokestatic has a caller").class.clone();
        let (class, method) = self.resolve_method(&from, index)?;
        let callee = &class.file.methods[method];
        if !callee.access_flags.has(AccessFlags::STATIC) {
            let name = external(&class.file.this_class, &callee.name, &callee.ty);
            let m = format!("Expected static method '{name}'");
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        // @hide
        self.initialize(&class)?;
        // @show
        let caller = self.frames.last_mut().expect("invokestatic has a caller");
        let args = caller.pop_args(callee.arg_slots())?;
        let frame = Frame::new(class.clone(), method, &args)?;
        self.call(frame)?;
        Ok(Flow::Next)
    }
    // @end

    // @listing call
    /// Pushes a new frame, unless the stack is full. While the VM constructs
    /// an exception, the stack may go `headroom` frames past the limit.
    pub(crate) fn call(&mut self, frame: Frame) -> Result<()> {
        if self.frames.len() >= MAX_FRAMES + self.headroom {
            let class = "java/lang/StackOverflowError";
            return Err(Error::Exception { class, message: None });
        }
        // @hide
        if frame.is_native() {
            self.calls += 1;
            return self.call_native(frame);
        }
        // @show
        self.frames.push(frame);
        self.calls += 1;
        self.deepest = self.deepest.max(self.frames.len());
        Ok(())
    }
    // @end
}

// @listing narrow-return
/// The value `ireturn` returns, narrowed to the method's return type: a
/// `byte`, `short` or `char` is truncated and sign- or zero-extended, and a
/// `boolean` keeps its lowest bit (JVMS 6.5.ireturn).
fn narrow_return(frame: &mut Frame) -> Result<Value> {
    let v = frame.pop_int()?;
    let ty = &frame.class.file.methods[frame.method].ty.ret;
    Ok(Value::Int(match ty {
        Some(FieldType::Boolean) => v & 1,
        Some(FieldType::Byte) => v as i8 as i32,
        Some(FieldType::Char) => v as u16 as i32,
        Some(FieldType::Short) => v as i16 as i32,
        _ => v,
    }))
}
// @end

impl Vm {
    // @listing object-op
    /// `new`, `getfield` and `putfield`.
    fn object_op(&mut self, opcode: u8, index: u16) -> Result<()> {
        let from = self.frames.last().expect("a frame").class.clone();
        if opcode == NEW {
            let name = from.file.constant_pool.class_name(index)?;
            let class = self.resolve_class(&from, name)?;
            let flags = class.file.access_flags;
            if flags.has(AccessFlags::ABSTRACT | AccessFlags::INTERFACE) {
                return Err(linkage("java/lang/InstantiationError", dotted(name)));
            }
            // @hide
            self.initialize(&class)?;
            // @show
            let object = self.heap.new_object(&class);
            return self.frames.last_mut().expect("a frame").push(Value::Ref(object));
        }
        let (owner, field) = self.resolve_field(&from, index)?;
        if field.access_flags.has(AccessFlags::STATIC) {
            let m = format!(
                "Expected non-static field {}.{}",
                dotted(&owner.file.this_class),
                field.name
            );
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        let slot = owner.field(&field.name, &field.descriptor)?;
        let frame = self.frames.last_mut().expect("a frame");
        if opcode == GETFIELD {
            let object = non_null(frame.pop_ref()?)?;
            frame.push(self.heap.read(object.0 + slot.offset, &slot.ty))
        } else {
            let value = frame.pop()?;
            let object = non_null(frame.pop_ref()?)?;
            self.heap.write(object.0 + slot.offset, &slot.ty, value)
        }
    }
    // @end

    // @listing invoke-special
    /// `invokespecial`: a constructor, a private method, or a superclass's
    /// version of a method (`super.m()`), with no selection by the object's
    /// class (JVMS 6.5).
    fn invoke_special(&mut self, index: u16) -> Result<Flow> {
        let from = self.frames.last().expect("a caller").class.clone();
        let target = from.file.constant_pool.member(index)?;
        let object_init =
            (target.class, target.name) == ("java/lang/Object", "<init>");
        if object_init && from.superclass.is_none() {
            // Object is a stand-in, and its constructor is empty.
            let caller = self.frames.last_mut().expect("a caller");
            non_null(caller.pop_ref()?)?;
            return Ok(Flow::Next);
        }
        let (class, method) = self.resolve_method(&from, index)?;
        // `super.m()`: start the search at the current class's superclass.
        let (class, method) = match &from.superclass {
            Some(superclass)
                if target.name != "<init>"
                    && is_subclass(&from, &class)
                    && !Rc::ptr_eq(&from, &class) =>
            {
                find_in_superclasses(superclass, target.name, target.descriptor)
                    .unwrap_or((class, method))
            }
            _ => (class, method),
        };
        let callee = &class.file.methods[method];
        let caller = self.frames.last_mut().expect("a caller");
        let args = caller.pop_args(callee.arg_slots())?;
        let Some(&Value::Ref(this)) = args.first() else {
            return Err(Error::Verify(format!("{} without an object", target.name)));
        };
        non_null(this)?;
        if callee.access_flags.has(AccessFlags::ABSTRACT) {
            let receiver = self.heap.class_of(this).clone();
            return Err(abstract_method(&receiver, &class, method));
        }
        let frame = Frame::new(class.clone(), method, &args)?;
        self.call(frame)?;
        Ok(Flow::Next)
    }
    // @end
}

// @listing non-null
/// `r`, or a `NullPointerException` if it is `null`.
pub(crate) fn non_null(r: Ref) -> Result<Ref> {
    if r == Ref::NULL {
        let class = "java/lang/NullPointerException";
        return Err(Error::Exception { class, message: None });
    }
    Ok(r)
}
// @end

impl Vm {
    // @listing invoke-virtual
    /// `invokevirtual`: resolves the method, then selects the one to run by
    /// the class of the object it is called on (JVMS 5.4.6).
    fn invoke_virtual(&mut self, index: u16) -> Result<Flow> {
        let from = self.frames.last().expect("a caller").class.clone();
        let (class, method) = self.resolve_method(&from, index)?;
        let resolved = &class.file.methods[method];
        if resolved.access_flags.has(AccessFlags::STATIC) {
            let name = external(&class.file.this_class, &resolved.name, &resolved.ty);
            let m = format!("Expecting non-static method '{name}'");
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        let caller = self.frames.last_mut().expect("a caller");
        let args = caller.pop_args(resolved.arg_slots())?;
        let Some(&Value::Ref(this)) = args.first() else {
            return Err(Error::Verify(format!(
                "{} without an object",
                resolved.name
            )));
        };
        let receiver = self.class_of(non_null(this)?)?;
        let (class, method) = match select(&receiver, &class, method) {
            Some((c, m))
                if !c.file.methods[m].access_flags.has(AccessFlags::ABSTRACT) =>
            {
                (c, m)
            }
            _ => return Err(abstract_method(&receiver, &class, method)),
        };
        self.call(Frame::new(class, method, &args)?)?;
        Ok(Flow::Next)
    }
    // @end
}

// @listing select
/// The method that an `invokevirtual` of method `mr` of `resolved` runs for
/// an object of class `receiver` (JVMS 5.4.6): `mr` itself if it is
/// private, and otherwise the first method found, from `receiver` up, that
/// is `mr` or can override it. Chapter 13 adds default methods.
pub fn select(
    receiver: &Rc<Class>,
    resolved: &Rc<Class>,
    mr: usize,
) -> Option<(Rc<Class>, usize)> {
    let m = &resolved.file.methods[mr];
    if m.access_flags.has(AccessFlags::PRIVATE) {
        return Some((resolved.clone(), mr));
    }
    let mut class = Some(receiver.clone());
    while let Some(c) = class {
        if let Ok(mc) = c.method(&m.name, &m.descriptor) {
            let same = Rc::ptr_eq(&c, resolved) && mc == mr;
            if same || overrides(&c, mc, resolved, mr) {
                return Some((c, mc));
            }
        }
        class = c.superclass.clone();
    }
    // @hide
    // A default method: the one maximally specific method that isn't abstract.
    let specific = maximally_specific(receiver, &m.name, &m.descriptor);
    let mut defaults = specific.into_iter().filter(|(c, method)| {
        !c.file.methods[*method].access_flags.has(AccessFlags::ABSTRACT)
    });
    if let (Some(default), None) = (defaults.next(), defaults.next()) {
        return Some(default);
    }
    // @show
    None
}

/// Whether method `mc` of `c` can override method `ma` of `a` (JVMS 5.4.5).
fn overrides(c: &Rc<Class>, mc: usize, a: &Rc<Class>, ma: usize) -> bool {
    let (method_c, method_a) = (&c.file.methods[mc], &a.file.methods[ma]);
    let flags = method_c.access_flags;
    if method_c.name != method_a.name || method_c.descriptor != method_a.descriptor {
        return false;
    }
    if flags.has(AccessFlags::PRIVATE | AccessFlags::STATIC) {
        return false;
    }
    if method_a.access_flags.has(AccessFlags::PUBLIC | AccessFlags::PROTECTED) {
        return true;
    }
    // Package-private: the same package, or through a class in between
    // whose method overrides `ma` and is overridden by `mc`.
    if package(&c.file.this_class) == package(&a.file.this_class) {
        return true;
    }
    let mut b = c.superclass.clone();
    while let Some(between) = b {
        if Rc::ptr_eq(&between, a) {
            break;
        }
        if let Ok(mb) = between.method(&method_a.name, &method_a.descriptor)
            && overrides(c, mc, &between, mb)
            && overrides(&between, mb, a, ma)
        {
            return true;
        }
        b = between.superclass.clone();
    }
    false
}

/// HotSpot's `AbstractMethodError` for a receiver that has no
/// implementation of method `mr` of `resolved`.
fn abstract_method(receiver: &Class, resolved: &Class, mr: usize) -> Error {
    let m = &resolved.file.methods[mr];
    let flags = resolved.file.access_flags;
    let kind = if flags.has(AccessFlags::INTERFACE) {
        "interface"
    } else if flags.has(AccessFlags::ABSTRACT) {
        "abstract class"
    } else {
        "class"
    };
    let ret = m.ty.ret.as_ref().map_or("void".to_string(), |t| t.to_string());
    let params: Vec<String> = m.ty.params.iter().map(|p| p.to_string()).collect();
    let message = format!(
        "Receiver class {} does not define or inherit an implementation of the \
         resolved method 'abstract {ret} {}({})' of {kind} {}.",
        dotted(&receiver.file.this_class),
        m.name,
        params.join(", "),
        dotted(&resolved.file.this_class)
    );
    linkage("java/lang/AbstractMethodError", message)
}
// @end

impl Vm {
    // @listing invoke-interface
    /// `invokeinterface`: resolves an interface method, checks that the
    /// object implements the interface, and selects as `invokevirtual` does.
    fn invoke_interface(&mut self, index: u16) -> Result<Flow> {
        let from = self.frames.last().expect("a caller").class.clone();
        let (class, method) = self.resolve_method(&from, index)?;
        let resolved = &class.file.methods[method];
        let caller = self.frames.last_mut().expect("a caller");
        let args = caller.pop_args(resolved.arg_slots())?;
        let Some(&Value::Ref(this)) = args.first() else {
            return Err(Error::Verify(format!(
                "{} without an object",
                resolved.name
            )));
        };
        let receiver = self.class_of(non_null(this)?)?;
        let interface = from.file.constant_pool.member(index)?.class;
        let interface = self.load(interface)?;
        if !is_subtype(&receiver, &interface) {
            let m = format!(
                "Class {} does not implement the requested interface {}",
                dotted(&receiver.file.this_class),
                dotted(&interface.file.this_class)
            );
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        let (class, method) = match select(&receiver, &class, method) {
            Some((c, m))
                if !c.file.methods[m].access_flags.has(AccessFlags::ABSTRACT) =>
            {
                (c, m)
            }
            _ => return Err(abstract_method(&receiver, &class, method)),
        };
        self.call(Frame::new(class, method, &args)?)?;
        Ok(Flow::Next)
    }
    // @end
}
