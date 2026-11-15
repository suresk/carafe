//! Chapter 4: frames, the operand stack and `int`.

use crate::attribute::Code;
use crate::bytecode::Operand;
use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::interpreter::Vm;
use crate::opcode::*;
use crate::pool::Constant;
use crate::value::Value;
use std::rc::Rc;

// @listing call-checked
/// Calls a static method from Rust after checking `args` against its
/// descriptor, so that a `long` can't be passed where a `double` is expected.
pub fn call_checked(
    vm: &mut Vm,
    class: &Rc<Class>,
    name: &str,
    descriptor: &str,
    args: &[Value],
) -> Result<Option<Value>> {
    let method = &class.file.methods[class.method(name, descriptor)?];
    if method.ty.params.len() != args.len() {
        let m =
            format!("{name}{descriptor} takes {} arguments", method.ty.params.len());
        return Err(Error::Verify(m));
    }
    for (ty, arg) in method.ty.params.iter().zip(args) {
        let fits = match ty {
            FieldType::Long => matches!(arg, Value::Long(_)),
            FieldType::Float => matches!(arg, Value::Float(_)),
            FieldType::Double => matches!(arg, Value::Double(_)),
            FieldType::Object(_) | FieldType::Array(_) => false,
            _ => matches!(arg, Value::Int(_)), // int, short, char, byte, boolean
        };
        if !fits {
            return Err(Error::Verify(format!("{arg:?} is not a {ty}")));
        }
    }
    vm.invoke(class, name, descriptor, args)
}
// @end

// @listing max-stack
/// The deepest the operand stack can get in `code`, in slots, found by
/// following every path through the method. Supports Chapter 4's
/// instructions.
pub fn max_stack(class: &Class, code: &Code) -> Result<usize> {
    let instructions = code.decode()?;
    let mut depth_at: Vec<Option<usize>> = vec![None; instructions.len()];
    let mut work = vec![(0, 0)]; // (instruction index, depth before it)
    let mut deepest = 0;
    while let Some((at, depth)) = work.pop() {
        match depth_at[at] {
            Some(seen) if seen == depth => continue,
            Some(seen) => {
                let m =
                    format!("depth {seen} and {depth} at {}", instructions[at].pc);
                return Err(Error::Verify(m));
            }
            None => depth_at[at] = Some(depth),
        }
        let i = &instructions[at];
        let (pops, pushes) = effect(class, i.opcode, &i.operand)?;
        let after = depth.checked_sub(pops).ok_or_else(|| {
            Error::Verify(format!("the stack underflows at {}", i.pc))
        })? + pushes;
        deepest = deepest.max(after);
        for target in i.targets() {
            let index = instructions.binary_search_by_key(&target, |i| i.pc);
            work.push((index.expect("decode checked targets"), after));
        }
        let ends = matches!(
            i.opcode,
            GOTO | GOTO_W | TABLESWITCH | LOOKUPSWITCH | IRETURN..=RETURN
        );
        if !ends && at + 1 < instructions.len() {
            work.push((at + 1, after));
        }
    }
    Ok(deepest)
}
// @end

// @listing effect
/// The slots an instruction pops and pushes.
fn effect(class: &Class, opcode: u8, operand: &Operand) -> Result<(usize, usize)> {
    Ok(match opcode {
        NOP | IINC | GOTO | GOTO_W | RETURN => (0, 0),
        ICONST_M1..=ICONST_5 | BIPUSH | SIPUSH | FCONST_0..=FCONST_2 => (0, 1),
        LCONST_0 | LCONST_1 | DCONST_0 | DCONST_1 => (0, 2),
        LDC | LDC_W | LDC2_W => {
            let &Operand::Constant(index) = operand else { unreachable!() };
            match class.file.constant_pool.get(index)? {
                Constant::Long(_) | Constant::Double(_) => (0, 2),
                _ => (0, 1),
            }
        }
        ILOAD | FLOAD | ALOAD | ILOAD_0..=ILOAD_3 => (0, 1),
        FLOAD_0..=FLOAD_3 | ALOAD_0..=ALOAD_3 => (0, 1),
        LLOAD | DLOAD | LLOAD_0..=LLOAD_3 | DLOAD_0..=DLOAD_3 => (0, 2),
        ISTORE | FSTORE | ASTORE | ISTORE_0..=ISTORE_3 => (1, 0),
        FSTORE_0..=FSTORE_3 | ASTORE_0..=ASTORE_3 => (1, 0),
        LSTORE | DSTORE | LSTORE_0..=LSTORE_3 | DSTORE_0..=DSTORE_3 => (2, 0),
        IADD | ISUB | IMUL | IDIV | IREM => (2, 1),
        ISHL | ISHR | IUSHR | IAND | IOR | IXOR => (2, 1),
        INEG => (1, 1),
        IFEQ..=IFLE | TABLESWITCH | LOOKUPSWITCH => (1, 0),
        IRETURN | FRETURN | ARETURN => (1, 0),
        LRETURN | DRETURN => (2, 0),
        IF_ICMPEQ..=IF_ICMPLE => (2, 0),
        POP => (1, 0),
        POP2 => (2, 0),
        DUP => (1, 2),
        DUP_X1 => (2, 3),
        DUP_X2 => (3, 4),
        DUP2 => (2, 4),
        DUP2_X1 => (3, 5),
        DUP2_X2 => (4, 6),
        SWAP => (2, 2),
        _ => {
            let m = format!("the stack effect of {}", MNEMONICS[usize::from(opcode)]);
            return Err(Error::Unsupported(m));
        }
    })
}
// @end
