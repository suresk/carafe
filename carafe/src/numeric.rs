//! `long`, `float` and `double` arithmetic, comparisons and conversions.

use crate::error::{Error, Result};
use crate::frame::Frame;
use crate::opcode::*;
use crate::value::Value;
use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Rem, Sub};

// @listing numeric
/// The constants, arithmetic, conversions and comparisons of `long`,
/// `float` and `double`, and the conversions from `int` (JVMS 2.8, 2.11.3–4).
pub(crate) fn numeric(frame: &mut Frame, op: u8) -> Result<()> {
    let value = match op {
        LCONST_0 | LCONST_1 => Value::Long((op - LCONST_0).into()),
        FCONST_0..=FCONST_2 => Value::Float((op - FCONST_0).into()),
        DCONST_0 | DCONST_1 => Value::Double((op - DCONST_0).into()),
        LSHL | LSHR | LUSHR => {
            let distance = frame.pop_int()? as u32;
            Value::Long(long_shift(op, frame.pop_long()?, distance))
        }
        LADD | LSUB | LMUL | LDIV | LREM | LAND | LOR | LXOR => {
            let b = frame.pop_long()?;
            Value::Long(long_op(op, frame.pop_long()?, b)?)
        }
        FADD | FSUB | FMUL | FDIV | FREM => {
            let b = frame.pop_float()?;
            Value::Float(float_op(op, frame.pop_float()?, b))
        }
        DADD | DSUB | DMUL | DDIV | DREM => {
            let b = frame.pop_double()?;
            Value::Double(float_op(op, frame.pop_double()?, b))
        }
        LNEG => Value::Long(frame.pop_long()?.wrapping_neg()),
        FNEG => Value::Float(-frame.pop_float()?),
        DNEG => Value::Double(-frame.pop_double()?),
        I2L..=I2S => convert(frame, op)?,
        LCMP => {
            let b = frame.pop_long()?;
            Value::Int(frame.pop_long()?.cmp(&b) as i32)
        }
        FCMPL | FCMPG => {
            let b = frame.pop_float()?;
            Value::Int(compare(frame.pop_float()?.partial_cmp(&b), op == FCMPG))
        }
        DCMPL | DCMPG => {
            let b = frame.pop_double()?;
            Value::Int(compare(frame.pop_double()?.partial_cmp(&b), op == DCMPG))
        }
        _ => unreachable!("numeric({op})"),
    };
    frame.push(value)
}
// @end

// @listing long-op
/// `long` arithmetic: as `int_op`, with 64 bits.
fn long_op(op: u8, a: i64, b: i64) -> Result<i64> {
    Ok(match op {
        LADD => a.wrapping_add(b),
        LSUB => a.wrapping_sub(b),
        LMUL => a.wrapping_mul(b),
        LDIV | LREM if b == 0 => {
            let message = Some("/ by zero".to_string());
            return Err(Error::Exception {
                class: "java/lang/ArithmeticException",
                message,
            });
        }
        LDIV => a.wrapping_div(b),
        LREM => a.wrapping_rem(b),
        LAND => a & b,
        LOR => a | b,
        _ => a ^ b,
    })
}

/// A `long` shift, by a distance taken modulo 64.
fn long_shift(op: u8, a: i64, distance: u32) -> i64 {
    match op {
        LSHL => a.wrapping_shl(distance),
        LSHR => a.wrapping_shr(distance),
        _ => (a as u64).wrapping_shr(distance) as i64,
    }
}
// @end

// @listing float-op
/// `f32` or `f64`: the arithmetic operators, with results of the same type.
trait Float:
    Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + Sized
{
}

impl Float for f32 {}
impl Float for f64 {}

/// `float` and `double` arithmetic, which is IEEE 754's: no exceptions, and
/// `%` truncates the quotient, as C's `fmod` does. `op` may be either the
/// `float` or the `double` opcode.
fn float_op<T: Float>(op: u8, a: T, b: T) -> T {
    match op {
        FADD | DADD => a + b,
        FSUB | DSUB => a - b,
        FMUL | DMUL => a * b,
        FDIV | DDIV => a / b,
        _ => a % b,
    }
}
// @end

// @listing convert
/// The conversions between primitive types, `i2l` to `i2s`. Rust's `as`
/// does what Java's casts do: float to integer rounds toward zero, saturates
/// at the integer's limits and makes NaN 0; integer to float rounds to nearest.
fn convert(frame: &mut Frame, op: u8) -> Result<Value> {
    Ok(match op {
        I2L => Value::Long(frame.pop_int()?.into()),
        I2F => Value::Float(frame.pop_int()? as f32),
        I2D => Value::Double(frame.pop_int()?.into()),
        L2I => Value::Int(frame.pop_long()? as i32),
        L2F => Value::Float(frame.pop_long()? as f32),
        L2D => Value::Double(frame.pop_long()? as f64),
        F2I => Value::Int(frame.pop_float()? as i32),
        F2L => Value::Long(frame.pop_float()? as i64),
        F2D => Value::Double(frame.pop_float()?.into()),
        D2I => Value::Int(frame.pop_double()? as i32),
        D2L => Value::Long(frame.pop_double()? as i64),
        D2F => Value::Float(frame.pop_double()? as f32),
        I2B => Value::Int(frame.pop_int()? as i8 as i32),
        I2C => Value::Int(frame.pop_int()? as u16 as i32),
        _ => Value::Int(frame.pop_int()? as i16 as i32), // I2S
    })
}
// @end

// @listing compare
/// `fcmpl`, `fcmpg`, `dcmpl` and `dcmpg`: -1, 0 or 1, and for a comparison
/// with NaN, which is unordered, -1 from the `l` forms and 1 from the `g`.
fn compare(order: Option<Ordering>, nan_is_greater: bool) -> i32 {
    match order {
        Some(order) => order as i32,
        None if nan_is_greater => 1,
        None => -1,
    }
}
// @end
