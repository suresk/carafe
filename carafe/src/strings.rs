//! Java strings: objects of the JDK's own `java.lang.String`.

use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::pool::Constant;
use crate::value::Value;

// @listing coders
/// `String.coder` for a string whose characters all fit in one byte.
pub const LATIN1: i32 = 0;
/// `String.coder` for any other string: two bytes per UTF-16 code unit.
pub const UTF16: i32 = 1;
// @end

impl Vm {
    // @listing new-string
    /// A new `java.lang.String` holding these UTF-16 code units, in the
    /// compact form the JDK uses: one byte per character if every character
    /// is below 256, two bytes per code unit, little-endian, otherwise.
    pub fn new_string(&mut self, units: &[u16]) -> Result<Ref> {
        let class = self.load("java/lang/String")?;
        let latin1 = units.iter().all(|&u| u <= 0xff);
        let bytes: Vec<u8> = if latin1 {
            units.iter().map(|&u| u as u8).collect()
        } else {
            units.iter().flat_map(|u| u.to_le_bytes()).collect()
        };
        let string = self.heap.new_object(&class);
        self.heap.roots.push(string); // held across the next allocation
        let array = self.heap.new_array(&FieldType::Byte, bytes.len() as u32);
        let string = self.heap.roots.pop().expect("pushed above");
        let array = array?;
        for (i, &b) in bytes.iter().enumerate() {
            let at = self.heap.element(array, i as i32)?;
            self.heap.write(at, &FieldType::Byte, Value::Int(b as i8 as i32))?;
        }
        let coder = if latin1 { LATIN1 } else { UTF16 };
        self.put(&class, string, "value", "[B", Value::Ref(array))?;
        self.put(&class, string, "coder", "B", Value::Int(coder))?;
        Ok(string)
    }

    /// Stores `value` in the instance field `name` of `object`.
    pub(crate) fn put(
        &mut self,
        class: &Class,
        object: Ref,
        name: &str,
        ty: &str,
        value: Value,
    ) -> Result<()> {
        let slot = class.field(name, ty)?;
        self.heap.write(object.0 + slot.offset, &slot.ty, value)
    }
    // @end

    // @listing intern
    /// The one `String` object for these code units: the same reference
    /// every time, as string literals are in Java (JLS 3.10.5).
    pub fn intern(&mut self, units: &[u16]) -> Result<Ref> {
        if let Some(&string) = self.strings.get(units) {
            return Ok(string);
        }
        let string = self.new_string(units)?;
        self.strings.insert(units.to_vec(), string);
        Ok(string)
    }

    /// `ldc` of a `String` constant: its text, interned.
    pub(crate) fn ldc_string(&mut self, index: u16) -> Result<Ref> {
        let class = self.frames.last().expect("a frame").class.clone();
        self.string_constant(&class, index)
    }

    /// The `String` for the constant at `index` in `class`'s pool.
    pub(crate) fn string_constant(
        &mut self,
        class: &Class,
        index: u16,
    ) -> Result<Ref> {
        let pool = &class.file.constant_pool;
        let Constant::String { utf8 } = *pool.get(index)? else { unreachable!() };
        let units: Vec<u16> = match pool.get(utf8)? {
            Constant::Utf8(text) => text.encode_utf16().collect(),
            Constant::Utf16(units) => units.clone(),
            other => return Err(Error::ClassFormat(format!("#{utf8} is {other:?}"))),
        };
        self.intern(&units)
    }
    // @end

    // @listing rust-string
    /// The text of the `String` at `r`, with any unpaired surrogate replaced
    /// by U+FFFD.
    pub fn rust_string(&self, r: Ref) -> Result<String> {
        let units = self.string_units(r)?;
        Ok(char::decode_utf16(units).map(|c| c.unwrap_or('\u{fffd}')).collect())
    }

    /// The UTF-16 code units of the `String` at `r`.
    pub fn string_units(&self, r: Ref) -> Result<Vec<u16>> {
        let class = self.heap.class_of(r);
        let field = |name: &str, ty: &str| -> Result<Value> {
            let slot = class.field(name, ty)?;
            Ok(self.heap.read(r.0 + slot.offset, &slot.ty))
        };
        let (Value::Ref(array), Value::Int(coder)) =
            (field("value", "[B")?, field("coder", "B")?)
        else {
            return Err(Error::Verify("a String without value and coder".into()));
        };
        let mut bytes = Vec::new();
        for i in 0..self.heap.array_length(array) as i32 {
            let at = self.heap.element(array, i)?;
            let Value::Int(b) = self.heap.read(at, &FieldType::Byte) else {
                unreachable!()
            };
            bytes.push(b as u8);
        }
        Ok(match coder {
            LATIN1 => bytes.iter().map(|&b| u16::from(b)).collect(),
            _ => bytes.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect(),
        })
    }
    // @end
}

// @listing is-string
/// Whether the constant at `index` in `class`'s pool is a `String`.
pub(crate) fn is_string(class: &Class, index: u16) -> bool {
    matches!(class.file.constant_pool.get(index), Ok(Constant::String { .. }))
}
// @end

impl Vm {
    // @listing string-intern
    /// `String.intern()`: the interned string with this one's contents, which
    /// is this one if no string with them has been interned.
    pub(crate) fn intern_native(
        vm: &mut Vm,
        args: &[Value],
    ) -> Result<Option<Value>> {
        let [Value::Ref(this)] = *args else { unreachable!("checked descriptor") };
        let units = vm.string_units(this)?;
        let interned = *vm.strings.entry(units).or_insert(this);
        Ok(Some(Value::Ref(interned)))
    }
    // @end
}
