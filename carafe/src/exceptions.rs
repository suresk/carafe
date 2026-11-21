//! Exceptions as Java objects: handlers, unwinding and stack traces.

use crate::class::Class;
use crate::error::{Error, Result};
use crate::frame::Frame;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::reader::Reader;
use crate::resolve::dotted;
use crate::value::Value;
use std::rc::Rc;

// @listing trace-frame
/// One line of a stack trace: a method, and the instruction it was executing,
/// which a native method doesn't have.
#[derive(Clone)]
pub struct TraceFrame {
    pub class: Rc<Class>,
    pub method: usize,
    pub pc: Option<u16>,
}

/// Two frames are equal when they show the same line of the same method,
/// as `StackTraceElement.equals` compares them, even at different pcs.
impl PartialEq for TraceFrame {
    fn eq(&self, other: &TraceFrame) -> bool {
        Rc::ptr_eq(&self.class, &other.class)
            && self.method == other.method
            && self.line_number() == other.line_number()
    }
}

/// HotSpot's `MaxJavaStackTraceDepth`: a trace keeps at most this many frames.
const MAX_TRACE: usize = 1024;

/// Frames the VM may push beyond `MAX_FRAMES` while it constructs an
/// exception, so that it can construct a `StackOverflowError`.
const HEADROOM: usize = 64;
// @end

impl Vm {
    // @listing catch
    /// Handles `error`, raised by the instruction the top frame just ran: makes
    /// it a Java object if the VM raised it, then pops frames above `depth`
    /// until one has a handler for it. If none has, the error comes back.
    pub(crate) fn catch(&mut self, error: Error, depth: usize) -> Result<()> {
        let exception = match error {
            Error::Thrown { exception, .. } => exception,
            // A VM without the JDK has no exception classes; and while the VM
            // makes one exception, a failure is not made into another.
            Error::Exception { class, message }
                if self.boot_dirs > 0 && self.headroom == 0 =>
            {
                self.exception(class, message)?
            }
            other => return Err(other),
        };
        self.heap.roots.push(exception);
        let caught = self.unwind(depth);
        let exception = self.heap.roots.pop().expect("pushed above");
        if caught? { Ok(()) } else { Err(self.thrown(exception)?) }
    }

    /// Pops frames above `depth` until one has a handler for the exception on
    /// top of `heap.roots`, and continues that frame at the handler, with the
    /// exception as the only thing on its operand stack.
    fn unwind(&mut self, depth: usize) -> Result<bool> {
        while self.frames.len() > depth {
            if let Some(next) = self.handler()? {
                let exception = *self.heap.roots.last().expect("the exception");
                let frame = self.frames.last_mut().expect("a frame");
                frame.stack.clear();
                frame.push(Value::Ref(exception))?;
                frame.next = next;
                return Ok(true);
            }
            // A method that exited its own monitor keeps the exception it
            // threw, as JVMS 2.11.10's structured locking allows.
            self.exit_method_monitor(self.frames.len() - 1).ok();
            self.frames.pop();
        }
        Ok(false)
    }
    // @end

    // @listing handler
    /// The index of the first instruction of the top frame's handler for the
    /// exception on top of `heap.roots`: the first entry of the exception
    /// table that covers the instruction and catches the exception's class.
    fn handler(&mut self) -> Result<Option<usize>> {
        let exception = *self.heap.roots.last().expect("the exception");
        let thrown = self.class_of(exception)?;
        let frame = self.frames.last().expect("a frame");
        let (class, code) = (frame.class.clone(), frame.code.clone());
        let pc = code[frame.next - 1].pc;
        let method = &class.file.methods[frame.method];
        let handlers = &method.code.as_ref().expect("a method with code").handlers;
        for h in handlers.iter().filter(|h| (h.start_pc..h.end_pc).contains(&pc)) {
            let catches = h.catch_type == 0 || {
                let name = class.file.constant_pool.class_name(h.catch_type)?;
                let catch_type = self.resolve_class(&class, name)?;
                self.is_assignable(&thrown, &catch_type)?
            };
            if catches {
                let next = code.binary_search_by_key(&h.handler_pc, |i| i.pc);
                let m =
                    format!("a handler at {}, inside an instruction", h.handler_pc);
                return next.map(Some).map_err(|_| Error::Verify(m));
            }
        }
        Ok(None)
    }
    // @end

    // @listing exception
    /// A new exception of class `class` with `message`, constructed as `new`
    /// and `<init>` would construct it, which also fills in its stack trace.
    pub(crate) fn exception(
        &mut self,
        class: &str,
        message: Option<String>,
    ) -> Result<Ref> {
        let saved = std::mem::replace(&mut self.headroom, HEADROOM);
        let exception = self.construct(class, message);
        self.headroom = saved;
        exception
    }

    fn construct(&mut self, class: &str, message: Option<String>) -> Result<Ref> {
        let class = self.load(class)?;
        self.initialize(&class)?;
        let exception = self.heap.new_object(&class);
        self.heap.roots.push(exception);
        let mut args = Vec::new();
        if let Some(message) = message {
            let units: Vec<u16> = message.encode_utf16().collect();
            args.push(Value::Ref(self.new_string(&units)?));
        }
        let descriptor =
            if args.is_empty() { "()V" } else { "(Ljava/lang/String;)V" };
        let exception = *self.heap.roots.last().expect("pushed above");
        args.insert(0, Value::Ref(exception));
        let init = class.method("<init>", descriptor)?;
        let result = self.invoke_method(&class, init, &args);
        let exception = self.heap.roots.pop().expect("pushed above");
        result.map(|_| exception)
    }

    /// Runs method `method` of `class`, static or not, and returns its result.
    pub(crate) fn invoke_method(
        &mut self,
        class: &Rc<Class>,
        method: usize,
        args: &[Value],
    ) -> Result<Option<Value>> {
        let depth = self.frames.len();
        self.call(Frame::new(class.clone(), method, args)?)?;
        let result =
            if self.frames.len() > depth { self.run(depth) } else { Ok(None) };
        self.frames.truncate(depth);
        result
    }
    // @end

    // @listing thrown
    /// `Error::Thrown` for `exception`, with its class and message.
    pub(crate) fn thrown(&mut self, exception: Ref) -> Result<Error> {
        let class = self.class_of(exception)?.file.this_class.clone();
        let message = match self.throwable_field(exception, "detailMessage")? {
            Ref::NULL => None,
            message => Some(self.rust_string(message)?),
        };
        Ok(Error::Thrown { exception, class, message })
    }

    /// A reference field of `Throwable`, such as `detailMessage` or `cause`.
    pub(crate) fn throwable_field(
        &mut self,
        exception: Ref,
        name: &str,
    ) -> Result<Ref> {
        let throwable = self.load("java/lang/Throwable")?;
        let ty = match name {
            "cause" => "Ljava/lang/Throwable;",
            "detailMessage" => "Ljava/lang/String;",
            _ => unreachable!("a reference field of Throwable: {name}"),
        };
        let slot = throwable.field(name, ty)?;
        match self.heap.read(exception.0 + slot.offset, &slot.ty) {
            Value::Ref(r) => Ok(r),
            _ => unreachable!("a reference field"),
        }
    }
    // @end
}

// @listing fill-in
/// `Throwable.fillInStackTrace(int)`: records the method calls in progress
/// as the exception's stack trace, without the ones that are making it,
/// which are `fillInStackTrace` itself and the constructors of the exception,
/// and sets `Throwable`'s `backtrace` and `depth`, which tell
/// `getStackTrace` that there is a trace to read.
pub(crate) fn fill_in_stack_trace(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(exception), _] = *args else {
        unreachable!("checked descriptor")
    };
    let class = vm.class_of(exception)?;
    let mut frames = vm.stack().into_iter().rev().peekable();
    let making =
        |f: &TraceFrame, name| f.name_is(name) && is_subclass(&class, &f.class);
    while frames.next_if(|f| making(f, "fillInStackTrace")).is_some() {}
    while frames.next_if(|f| making(f, "<init>")).is_some() {}
    let trace: Vec<TraceFrame> = frames.take(MAX_TRACE).collect();
    let throwable = vm.load("java/lang/Throwable")?;
    let depth = Value::Int(trace.len() as i32);
    vm.put(&throwable, exception, "depth", "I", depth)?;
    // Any object will do as the marker; the trace itself stays in Rust.
    let marker = Value::Ref(exception);
    vm.put(&throwable, exception, "backtrace", "Ljava/lang/Object;", marker)?;
    vm.backtraces.insert(exception, trace);
    Ok(Some(Value::Ref(exception)))
}

impl Vm {
    /// The method calls in progress, bottom first: each frame at the
    /// instruction it is executing, and each native that is running above
    /// the frame that called it.
    fn stack(&self) -> Vec<TraceFrame> {
        let mut natives = self.native_frames.iter().peekable();
        let mut stack = Vec::new();
        for (i, frame) in self.frames.iter().enumerate() {
            while let Some((_, native)) = natives.next_if(|(at, _)| *at <= i) {
                stack.push(native.clone());
            }
            // A frame that hasn't started yet is at its first instruction.
            let pc = frame.next.checked_sub(1).map_or(0, |n| frame.code[n].pc);
            let (class, method) = (frame.class.clone(), frame.method);
            stack.push(TraceFrame { class, method, pc: Some(pc) });
        }
        stack.extend(natives.map(|(_, native)| native.clone()));
        stack
    }

    /// An error that a native returned, made an exception object while the
    /// native is still on the stack, so that the trace starts with it.
    pub(crate) fn native_error(&mut self, error: Error) -> Error {
        match error {
            Error::Exception { class, message }
                if self.boot_dirs > 0 && self.headroom == 0 =>
            {
                let thrown = self.exception(class, message);
                match thrown.and_then(|exception| self.thrown(exception)) {
                    Ok(error) | Err(error) => error,
                }
            }
            other => other,
        }
    }
}

/// Whether `class` is `ancestor` or a subclass of it.
fn is_subclass(class: &Rc<Class>, ancestor: &Rc<Class>) -> bool {
    let mut class = Some(class.clone());
    while let Some(c) = class {
        if Rc::ptr_eq(&c, ancestor) {
            return true;
        }
        class = c.superclass.clone();
    }
    false
}
// @end

// @listing stack-trace-elements
/// `StackTraceElement.initStackTraceElements(elements, backtrace, depth)`:
/// fills in the elements that `getStackTrace` made from the trace that
/// `fill_in_stack_trace` recorded, under the exception it left in
/// `backtrace`. Java's `computeFormat` then reads `declaringClassObject`.
pub(crate) fn init_stack_trace_elements(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(elements), Value::Ref(backtrace), Value::Int(depth)] = *args
    else {
        unreachable!("checked descriptor")
    };
    let trace = vm.backtraces.get(&backtrace).cloned().unwrap_or_default();
    let elements = crate::interpreter::non_null(elements)?;
    let count = vm.heap.array_length(elements) as usize;
    let count = count.min(trace.len()).min(usize::try_from(depth).unwrap_or(0));
    vm.heap.roots.push(elements); // held across the allocations below
    let filled = trace[..count]
        .iter()
        .enumerate()
        .try_for_each(|(i, frame)| init_element(vm, i as i32, frame));
    vm.heap.roots.pop();
    filled.map(|_| None)
}

/// Fills in element `i` of the array on top of `heap.roots` from `frame`.
fn init_element(vm: &mut Vm, i: i32, frame: &TraceFrame) -> Result<()> {
    let class = vm.load("java/lang/StackTraceElement")?;
    let set = |vm: &mut Vm, name: &str, ty: &str, value: Value| {
        let array = *vm.heap.roots.last().expect("pushed by the caller");
        let element = crate::unsafe_access::object();
        let Value::Ref(element) = vm.heap.read(vm.heap.element(array, i)?, &element)
        else {
            unreachable!("a StackTraceElement[]")
        };
        vm.put(&class, element, name, ty, value)
    };
    let mirror = vm.mirror(&frame.class)?;
    set(vm, "declaringClassObject", "Ljava/lang/Class;", Value::Ref(mirror))?;
    let method = &frame.class.file.methods[frame.method];
    let strings = [
        ("declaringClass", Some(dotted(&frame.class.file.this_class))),
        ("methodName", Some(method.name.clone())),
        ("fileName", source_file(&frame.class)),
        ("moduleName", frame.class.boot.then(|| "java.base".to_string())),
    ];
    for (name, text) in strings {
        if let Some(text) = text {
            let units: Vec<u16> = text.encode_utf16().collect();
            let string = vm.new_string(&units)?;
            set(vm, name, "Ljava/lang/String;", Value::Ref(string))?;
        }
    }
    // -2 marks a native method, and -1 an unknown line.
    let line = match frame.pc {
        None => -2,
        Some(_) => frame.line_number().map_or(-1, i32::from),
    };
    set(vm, "lineNumber", "I", Value::Int(line))
}
// @end

// @listing name-is
impl TraceFrame {
    /// Whether this frame's method is called `name`.
    fn name_is(&self, name: &str) -> bool {
        self.class.file.methods[self.method].name == name
    }
}
// @end

// @listing trace-line
impl TraceFrame {
    /// The frame as `printStackTrace` prints it: the class, the method, the
    /// source file and line, and the module for a class on the boot class
    /// path, which is all `java.base`.
    pub fn line(&self) -> String {
        let name = dotted(&self.class.file.this_class);
        let module = if self.class.boot { "java.base/" } else { "" };
        let method = &self.class.file.methods[self.method].name;
        let place = match (source_file(&self.class), self.line_number()) {
            _ if self.pc.is_none() => "Native Method".to_string(),
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file,
            _ => "Unknown Source".to_string(),
        };
        format!("\tat {module}{name}.{method}({place})")
    }

    /// The source line of the instruction, from the `LineNumberTable`s: the
    /// line of the first entry with the greatest start that is not after it.
    pub(crate) fn line_number(&self) -> Option<u16> {
        let pc = self.pc?;
        let code = self.class.file.methods[self.method].code.as_ref()?;
        let mut best: Option<(u16, u16)> = None;
        for table in code.attributes.iter().filter(|a| a.name == "LineNumberTable") {
            let mut r = Reader::new(&table.info);
            for _ in 0..r.u2().ok()? {
                let (start, line) = (r.u2().ok()?, r.u2().ok()?);
                if start <= pc && best.is_none_or(|(s, _)| start > s) {
                    best = Some((start, line));
                }
            }
        }
        best.map(|(_, line)| line)
    }
}

/// The name in the class's `SourceFile` attribute, such as `Hello.java`.
fn source_file(class: &Class) -> Option<String> {
    let attribute = class.file.attributes.iter().find(|a| a.name == "SourceFile")?;
    let index = Reader::new(&attribute.info).u2().ok()?;
    Some(class.file.constant_pool.utf8(index).ok()?.to_string())
}
// @end

impl Vm {
    // @listing report
    /// What HotSpot prints when `exception` ends the program: `Exception in
    /// thread "main"`, the exception and its stack trace, then each cause
    /// with the frames it doesn't share with the one before, until a cause
    /// that was printed already.
    pub fn report(&mut self, exception: Ref) -> Result<String> {
        let mut out = String::from("Exception in thread \"main\" ");
        let mut exception = exception;
        let mut enclosing: Vec<TraceFrame> = Vec::new();
        let mut printed = Vec::new();
        loop {
            printed.push(exception);
            out += &self.java_to_string(exception)?;
            out += "\n";
            let trace = self.backtraces.get(&exception).cloned().unwrap_or_default();
            let shared = trace.iter().rev().zip(enclosing.iter().rev());
            let common = shared.take_while(|(a, b)| a == b).count();
            for frame in &trace[..trace.len() - common] {
                out += &frame.line();
                out += "\n";
            }
            if common > 0 {
                out += &format!("\t... {common} more\n");
            }
            let cause = self.java_cause(exception)?;
            if cause == Ref::NULL {
                return Ok(out);
            }
            out += "Caused by: ";
            if printed.contains(&cause) {
                let cause = self.java_to_string(cause)?;
                return Ok(out + &format!("[CIRCULAR REFERENCE: {cause}]\n"));
            }
            (exception, enclosing) = (cause, trace);
        }
    }

    /// `object.toString()`, run in Java.
    pub(crate) fn java_to_string(&mut self, object: Ref) -> Result<String> {
        let string = self.call_virtual(
            object,
            "java/lang/Object",
            "toString",
            "()Ljava/lang/String;",
        );
        match string? {
            Some(Value::Ref(s)) if s != Ref::NULL => self.rust_string(s),
            _ => Ok("null".to_string()),
        }
    }

    /// `exception.getCause()`, run in Java, which a subclass may override, as
    /// `InvocationTargetException` does.
    pub(crate) fn java_cause(&mut self, exception: Ref) -> Result<Ref> {
        let cause = "()Ljava/lang/Throwable;";
        match self.call_virtual(
            exception,
            "java/lang/Throwable",
            "getCause",
            cause,
        )? {
            Some(Value::Ref(cause)) => Ok(cause),
            _ => Ok(Ref::NULL),
        }
    }

    /// Calls the method `name` of `owner` on `object`, selected from the
    /// object's class as `invokevirtual` selects it.
    fn call_virtual(
        &mut self,
        object: Ref,
        owner: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<Option<Value>> {
        let class = self.class_of(object)?;
        let owner = self.load(owner)?;
        let method = owner.method(name, descriptor)?;
        let Some((class, method)) =
            crate::interpreter::select(&class, &owner, method)
        else {
            let m = format!("{}.{name}{descriptor}", dotted(&class.file.this_class));
            return Err(linkage("java/lang/AbstractMethodError", m));
        };
        self.invoke_method(&class, method, &[Value::Ref(object)])
    }
    // @end
}

impl Vm {
    // @listing in-initializer
    /// JVMS 5.5, step 11, for an exception object: one that isn't an `Error`
    /// becomes the cause of a new `ExceptionInInitializerError`.
    pub(crate) fn in_initializer(&mut self, error: Error) -> Error {
        let Error::Thrown { exception, .. } = error else { return error };
        self.wrap(exception).unwrap_or_else(|failure| failure)
    }

    fn wrap(&mut self, exception: Ref) -> Result<Error> {
        let (class, error) =
            (self.class_of(exception)?, self.load("java/lang/Error")?);
        if self.is_assignable(&class, &error)? {
            return self.thrown(exception);
        }
        self.heap.roots.push(exception);
        let wrapper = self.exception("java/lang/ExceptionInInitializerError", None);
        let exception = self.heap.roots.pop().expect("pushed above");
        let (wrapper, throwable) = (wrapper?, self.load("java/lang/Throwable")?);
        self.put(
            &throwable,
            wrapper,
            "cause",
            "Ljava/lang/Throwable;",
            Value::Ref(exception),
        )?;
        self.thrown(wrapper)
    }
    // @end
}
