//! Chapter 6: long, float, double and the rules of Java arithmetic.

use crate::class_file::ClassFile;
use crate::error::Result;
use crate::opcode::*;

// @listing float-comparisons
/// The (comparison, branch) pairs in a method: each `fcmpl`, `fcmpg`,
/// `dcmpl` or `dcmpg` with the conditional branch that uses its result.
pub fn float_comparisons(
    class: &ClassFile,
    name: &str,
) -> Result<Vec<(String, String)>> {
    let method = class.methods.iter().find(|m| m.name == name);
    let Some(code) = method.and_then(|m| m.code.as_ref()) else {
        return Ok(Vec::new());
    };
    let instructions = code.decode()?;
    let pairs = instructions.windows(2).filter(|w| {
        matches!(w[0].opcode, FCMPL..=DCMPG) && matches!(w[1].opcode, IFEQ..=IFLE)
    });
    Ok(pairs.map(|w| (w[0].mnemonic(), w[1].mnemonic())).collect())
}
// @end

// @listing f2i-by-hand
/// `f2i` without `as`: from the bits of an IEEE 754 single, round toward
/// zero, saturate at `int`'s limits, and make NaN 0.
pub fn f2i_by_hand(x: f32) -> i32 {
    let bits = x.to_bits();
    let negative = bits >> 31 == 1;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127; // unbiased
    let fraction = bits & 0x7f_ffff;
    if exponent == 128 && fraction != 0 {
        return 0; // NaN
    }
    if exponent < 0 {
        return 0; // |x| < 1, including zeros and subnormals
    }
    if exponent >= 31 {
        // |x| >= 2^31: out of range, or exactly i32::MIN
        return if negative { i32::MIN } else { i32::MAX };
    }
    let significand = (fraction | 0x80_0000) as i64; // the implicit leading 1
    let magnitude = if exponent >= 23 {
        significand << (exponent - 23)
    } else {
        significand >> (23 - exponent) // drops the fraction: toward zero
    };
    (if negative { -magnitude } else { magnitude }) as i32
}
// @end
