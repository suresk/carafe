//! A frame: one method call's local variables and operand stack.

use crate::bytecode::Instruction;
use crate::class::Class;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::heap::Ref;
use crate::value::Value;
use std::rc::Rc;

// @listing frame
/// The state of one method call (JVMS 2.6).
pub struct Frame {
    pub class: Rc<Class>,
    /// The method's index in `class.file.methods`.
    pub method: usize,
    pub code: Rc<[Instruction]>,
    /// The index in `code` of the next instruction to execute.
    pub next: usize,
    pub locals: Vec<Value>,
    /// The operand stack, one entry per slot: a `long` is `Long`, then `Top`.
    pub stack: Vec<Value>,
    max_stack: usize,
}
// @end

impl Frame {
    // @listing frame-new
    /// A frame for calling method `method` of `class`, with `args` in the
    /// first local variables.
    pub fn new(class: Rc<Class>, method: usize, args: &[Value]) -> Result<Frame> {
        // @hide
        if class.file.methods[method].access_flags.has(AccessFlags::NATIVE) {
            return Ok(Frame::native(class, method, args));
        }
        // @show
        let code = class.instructions(method)?;
        let m = &class.file.methods[method];
        let attribute = m.code.as_ref().expect("instructions() found code");
        let mut locals = vec![Value::Top; attribute.max_locals.into()];
        let mut slot = 0;
        for &arg in args {
            let width = if arg.is_wide() { 2 } else { 1 };
            let Some(place) = locals.get_mut(slot..slot + width) else {
                return Err(verify(
                    m.name.as_str(),
                    "has more arguments than locals",
                ));
            };
            place[0] = arg;
            slot += width;
        }
        if slot != m.arg_slots() {
            let given =
                format!("was given {slot} slots of arguments, not {}", m.arg_slots());
            return Err(verify(m.name.as_str(), &given));
        }
        let max_stack = attribute.max_stack.into();
        let stack = Vec::with_capacity(max_stack);
        Ok(Frame { class, method, code, next: 0, locals, stack, max_stack })
    }
    // @end

    // @listing frame-stack
    /// Pushes a value, and `Top` after a `long` or a `double`.
    pub fn push(&mut self, value: Value) -> Result<()> {
        let width = if value.is_wide() { 2 } else { 1 };
        if self.stack.len() + width > self.max_stack {
            return Err(verify(self.name(), "overflows its max_stack"));
        }
        self.stack.push(value);
        if value.is_wide() {
            self.stack.push(Value::Top);
        }
        Ok(())
    }

    /// Pops a value, both slots of it if it is a `long` or a `double`.
    pub fn pop(&mut self) -> Result<Value> {
        match self.pop_slot()? {
            Value::Top => match self.pop_slot()? {
                wide if wide.is_wide() => Ok(wide),
                _ => Err(verify(self.name(), "pops half of a value")),
            },
            value => Ok(value),
        }
    }

    pub fn pop_int(&mut self) -> Result<i32> {
        match self.pop()? {
            Value::Int(v) => Ok(v),
            other => {
                Err(verify(self.name(), &format!("expected an int, found {other:?}")))
            }
        }
    }

    /// Pushes one slot, which may be half of a `long` or a `double`.
    pub fn push_slot(&mut self, slot: Value) -> Result<()> {
        if self.stack.len() == self.max_stack {
            return Err(verify(self.name(), "overflows its max_stack"));
        }
        self.stack.push(slot);
        Ok(())
    }

    /// Pops one slot, which may be half of a `long` or a `double`.
    pub fn pop_slot(&mut self) -> Result<Value> {
        match self.stack.pop() {
            Some(value) => Ok(value),
            None => Err(verify(self.name(), "pops an empty operand stack")),
        }
    }
    // @end

    // @listing frame-locals
    /// The value in local variable `n`.
    pub fn load(&self, n: u16) -> Result<Value> {
        match self.locals.get(usize::from(n)) {
            Some(Value::Top) | None => {
                Err(verify(self.name(), &format!("loads local {n}")))
            }
            Some(&value) => Ok(value),
        }
    }

    /// Stores `value` in local variable `n`, and `n + 1` if it is wide.
    pub fn store(&mut self, n: u16, value: Value) -> Result<()> {
        let n = usize::from(n);
        let width = if value.is_wide() { 2 } else { 1 };
        let Some(place) = self.locals.get_mut(n..n + width) else {
            return Err(verify(self.name(), &format!("stores past local {n}")));
        };
        place[0] = value;
        if width == 2 {
            place[1] = Value::Top;
        }
        Ok(())
    }

    /// The name of the frame's method, for error messages.
    pub fn name(&self) -> &str {
        &self.class.file.methods[self.method].name
    }
}

fn verify(method: &str, problem: &str) -> Error {
    Error::Verify(format!("{method} {problem}"))
}
// @end

impl Frame {
    // @listing frame-pops
    pub fn pop_long(&mut self) -> Result<i64> {
        match self.pop()? {
            Value::Long(v) => Ok(v),
            other => Err(self.expected("a long", other)),
        }
    }

    pub fn pop_float(&mut self) -> Result<f32> {
        match self.pop()? {
            Value::Float(v) => Ok(v),
            other => Err(self.expected("a float", other)),
        }
    }

    pub fn pop_double(&mut self) -> Result<f64> {
        match self.pop()? {
            Value::Double(v) => Ok(v),
            other => Err(self.expected("a double", other)),
        }
    }

    fn expected(&self, what: &str, found: Value) -> Error {
        verify(self.name(), &format!("expected {what}, found {found:?}"))
    }
    // @end
}

impl Frame {
    // @listing pop-args
    /// Pops the arguments of a call, `slots` slots of them, and returns them
    /// in the order they were pushed.
    pub fn pop_args(&mut self, slots: usize) -> Result<Vec<Value>> {
        let mut args = Vec::new();
        let mut popped = 0;
        while popped < slots {
            let arg = self.pop()?;
            popped += if arg.is_wide() { 2 } else { 1 };
            args.push(arg);
        }
        args.reverse();
        Ok(args)
    }
    // @end
}

impl Frame {
    // @listing frame-pop-ref
    pub fn pop_ref(&mut self) -> Result<Ref> {
        match self.pop()? {
            Value::Ref(r) => Ok(r),
            other => Err(self.expected("a reference", other)),
        }
    }
    // @end
}

impl Frame {
    // @listing frame-native
    /// A frame for a native method: no code, and its arguments as locals,
    /// for the native to read.
    fn native(class: Rc<Class>, method: usize, args: &[Value]) -> Frame {
        let mut locals = Vec::new();
        for &arg in args {
            locals.push(arg);
            if arg.is_wide() {
                locals.push(Value::Top);
            }
        }
        let code = Rc::from([]);
        Frame {
            class,
            method,
            code,
            next: 0,
            locals,
            stack: Vec::new(),
            max_stack: 0,
        }
    }

    /// Whether this is a frame for a native method.
    pub fn is_native(&self) -> bool {
        self.class.file.methods[self.method].access_flags.has(AccessFlags::NATIVE)
    }

    /// The arguments of a call, one value each: the locals without the
    /// second halves of `long`s and `double`s.
    pub fn args(&self) -> Vec<Value> {
        self.locals.iter().copied().filter(|v| *v != Value::Top).collect()
    }
    // @end
}
