//! Exceptions as Java objects: handlers, unwinding and stack traces.

use crate::class::Class;
use crate::error::{Error, Result};
use crate::frame::Frame;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::reader::Reader;
use crate::resolve::dotted;
use crate::value::Value;
use std::rc::Rc;

// @listing trace-frame
/// One line of a stack trace: a method, and the instruction it was executing.
#[derive(Clone)]
pub struct TraceFrame {
    pub class: Rc<Class>,
    pub method: usize,
    pub pc: u16,
}

impl PartialEq for TraceFrame {
    fn eq(&self, other: &TraceFrame) -> bool {
        Rc::ptr_eq(&self.class, &other.class)
            && (self.method, self.pc) == (other.method, other.pc)
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
        match caught? {
            true => Ok(()),
            false => Err(self.thrown(exception)?),
        }
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
        self.headroom = HEADROOM;
        let exception = self.construct(class, message);
        self.headroom = 0;
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
        let ty = if name == "cause" {
            "Ljava/lang/Throwable;"
        } else {
            "Ljava/lang/String;"
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
/// `Throwable.fillInStackTrace(int)`: records the frames on the stack as the
/// exception's stack trace, without the frames that are making it, which
/// are `fillInStackTrace` itself and the constructors of the exception.
pub(crate) fn fill_in_stack_trace(
    vm: &mut Vm,
    args: &[Value],
) -> Result<Option<Value>> {
    let [Value::Ref(exception), _] = *args else {
        unreachable!("checked descriptor")
    };
    let class = vm.class_of(exception)?;
    let mut frames = vm.frames.iter().rev().peekable();
    while frames.next_if(|f| f.name_is("fillInStackTrace")).is_some() {}
    let mut trace = Vec::new();
    let mut constructors = true;
    for frame in frames {
        if constructors
            && frame.name_is("<init>")
            && is_subclass(&class, &frame.class)
        {
            continue;
        }
        constructors = false;
        let pc = frame.code[frame.next - 1].pc;
        trace.push(TraceFrame {
            class: frame.class.clone(),
            method: frame.method,
            pc,
        });
        if trace.len() == MAX_TRACE {
            break;
        }
    }
    vm.backtraces.insert(exception, trace);
    Ok(Some(Value::Ref(exception)))
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

impl Frame {
    /// Whether this frame's method is called `name`.
    pub(crate) fn name_is(&self, name: &str) -> bool {
        self.class.file.methods[self.method].name == name
    }
}

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
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file,
            _ => "Unknown Source".to_string(),
        };
        format!("\tat {module}{name}.{method}({place})")
    }

    /// The source line of the instruction, from the `LineNumberTable`s: the
    /// line of the entry with the greatest start that is not after it.
    fn line_number(&self) -> Option<u16> {
        let code = self.class.file.methods[self.method].code.as_ref()?;
        let mut best: Option<(u16, u16)> = None;
        for table in code.attributes.iter().filter(|a| a.name == "LineNumberTable") {
            let mut r = Reader::new(&table.info);
            for _ in 0..r.u2().ok()? {
                let (start, line) = (r.u2().ok()?, r.u2().ok()?);
                if start <= self.pc && best.is_none_or(|(s, _)| start >= s) {
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
    /// with the frames it doesn't share with the one before.
    pub fn report(&mut self, exception: Ref) -> Result<String> {
        let mut out = String::from("Exception in thread \"main\" ");
        let mut exception = exception;
        let mut enclosing: Vec<TraceFrame> = Vec::new();
        loop {
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
            if cause == Ref::NULL || cause == exception {
                return Ok(out);
            }
            out += "Caused by: ";
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
            _ => unreachable!("getCause returns a Throwable"),
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
        let (class, method) =
            crate::interpreter::select(&class, &owner, method).expect("a method");
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
