//! The values the interpreter computes with.

use crate::heap::Ref;

// @listing value
/// A value in a local variable or on the operand stack. A `long` or a
/// `double` fills two slots: the value, then `Top`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// No value: the second slot of a `long` or `double`, or a local
    /// variable that hasn't been assigned (the JVMS's verification type `top`).
    Top,
    // @hide
    /// A reference to an object, or `null`.
    Ref(Ref),
    // @show
}

impl Value {
    /// Whether the value fills two slots.
    pub fn is_wide(self) -> bool {
        matches!(self, Value::Long(_) | Value::Double(_))
    }
}
// @end
