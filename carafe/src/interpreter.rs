//! The interpreter: a loop that executes one instruction at a time.

use crate::bytecode::{Instruction, Operand};
use crate::class::Class;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::frame::Frame;
use crate::opcode::*;
use crate::pool::Constant;
use crate::value::Value;
use std::rc::Rc;

// @listing vm
/// The virtual machine: the stack of frames of the method calls in
/// progress, and a count of the instructions executed so far.
#[derive(Default)]
pub struct Vm {
    frames: Vec<Frame>,
    pub executed: u64,
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
    /// Executes instructions until the frame at `depth` returns.
    fn run(&mut self, depth: usize) -> Result<Option<Value>> {
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
            match self.execute(instruction)? {
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
