//! The heap: every object, laid out as bytes in one `Vec<u8>`.

use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::value::Value;
use std::rc::Rc;

// @listing ref
/// A reference: the offset of an object in the heap. Offset 0 is never an
/// object, so it is `null`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ref(pub u32);

impl Ref {
    pub const NULL: Ref = Ref(0);
}
// @end

// @listing heap
/// The bytes of every object, and the classes the objects' headers name.
/// An object is an 8-byte header, the index of its class and its identity
/// hash, followed by its fields. Every object starts at a multiple of 8.
pub struct Heap {
    bytes: Vec<u8>,
    classes: Vec<Rc<Class>>,
    // @hide
    /// The element types of arrays, which an array's header names.
    element_types: Vec<FieldType>,
    /// References that Rust code is holding across an allocation (Section 10.5).
    pub roots: Vec<Ref>,
    // @show
}

/// The size of an object's header, in bytes.
pub const HEADER: u32 = 8;

impl Default for Heap {
    fn default() -> Heap {
        // The first 8 bytes are never an object, so that offset 0 can be null.
        Heap {
            bytes: vec![0; 8],
            classes: Vec::new(),
            // @hide
            element_types: Vec::new(),
            roots: Vec::new(),
            // @show
        }
    }
}
// @end

impl Heap {
    // @listing new-object
    /// Allocates an object of `class` with every field zero (`0`, `0.0`,
    /// `false`, `null`), as the JVMS requires of `new`.
    pub fn new_object(&mut self, class: &Rc<Class>) -> Ref {
        let id = match self.classes.iter().position(|c| Rc::ptr_eq(c, class)) {
            Some(id) => id,
            None => {
                self.classes.push(class.clone());
                self.classes.len() - 1
            }
        };
        let at = self.bytes.len();
        let size = class.layout.size as usize;
        self.bytes.resize(at + size.next_multiple_of(8), 0);
        let r = Ref(u32::try_from(at).expect("the heap is under 4 GB"));
        self.put_u32(at, id as u32);
        self.put_u32(at + 4, r.0); // the identity hash: the first address
        r
    }

    /// The class of the object at `r`.
    pub fn class_of(&self, r: Ref) -> &Rc<Class> {
        &self.classes[self.u32_at(r.0 as usize) as usize]
    }

    /// The object's identity hash code, which `Object.hashCode` returns.
    pub fn hash(&self, r: Ref) -> u32 {
        self.u32_at(r.0 as usize + 4)
    }

    /// The bytes in use, including the 8 that are never an object.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    // @end

    // @listing read-write
    /// Reads the value of type `ty` at offset `at`.
    pub fn read(&self, at: u32, ty: &FieldType) -> Value {
        let at = at as usize;
        let b = &self.bytes[at..at + size_of(ty) as usize];
        match ty {
            FieldType::Boolean | FieldType::Byte => Value::Int(b[0] as i8 as i32),
            FieldType::Char => Value::Int(u16::from_le_bytes([b[0], b[1]]) as i32),
            FieldType::Short => Value::Int(i16::from_le_bytes([b[0], b[1]]) as i32),
            FieldType::Int => Value::Int(i32::from_le_bytes(array(b))),
            FieldType::Float => Value::Float(f32::from_le_bytes(array(b))),
            FieldType::Long => Value::Long(i64::from_le_bytes(array(b))),
            FieldType::Double => Value::Double(f64::from_le_bytes(array(b))),
            FieldType::Object(_) | FieldType::Array(_) => {
                Value::Ref(Ref(u32::from_le_bytes(array(b))))
            }
        }
    }

    /// Writes `value` as type `ty` at offset `at`. An `int` stored as a
    /// narrower type keeps its low bytes; a `boolean` keeps its lowest bit.
    pub fn write(&mut self, at: u32, ty: &FieldType, value: Value) -> Result<()> {
        let at = at as usize;
        match (ty, value) {
            (FieldType::Boolean, Value::Int(v)) => self.put(at, &[(v & 1) as u8]),
            (FieldType::Byte, Value::Int(v)) => self.put(at, &[v as u8]),
            (FieldType::Char | FieldType::Short, Value::Int(v)) => {
                self.put(at, &(v as u16).to_le_bytes())
            }
            (FieldType::Int, Value::Int(v)) => self.put(at, &v.to_le_bytes()),
            (FieldType::Float, Value::Float(v)) => self.put(at, &v.to_le_bytes()),
            (FieldType::Long, Value::Long(v)) => self.put(at, &v.to_le_bytes()),
            (FieldType::Double, Value::Double(v)) => self.put(at, &v.to_le_bytes()),
            (FieldType::Object(_) | FieldType::Array(_), Value::Ref(r)) => {
                self.put(at, &r.0.to_le_bytes())
            }
            (ty, value) => {
                return Err(Error::Verify(format!("storing {value:?} as {ty}")));
            }
        }
        Ok(())
    }

    fn put(&mut self, at: usize, bytes: &[u8]) {
        self.bytes[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn u32_at(&self, at: usize) -> u32 {
        u32::from_le_bytes(array(&self.bytes[at..at + 4]))
    }

    fn put_u32(&mut self, at: usize, v: u32) {
        self.put(at, &v.to_le_bytes());
    }
}

/// A slice as an array of `N` bytes; the caller has sliced exactly `N`.
fn array<const N: usize>(b: &[u8]) -> [u8; N] {
    b.try_into().expect("a slice of N bytes")
}

/// The bytes a value of type `ty` takes in an object: a reference is 4.
pub fn size_of(ty: &FieldType) -> u32 {
    match ty {
        FieldType::Boolean | FieldType::Byte => 1,
        FieldType::Char | FieldType::Short => 2,
        FieldType::Long | FieldType::Double => 8,
        _ => 4, // int, float, and references
    }
}
// @end

// @listing array-header
/// The bit that marks an array in the first word of a header, whose other
/// bits are then the index of the array's element type.
pub const ARRAY: u32 = 1 << 31;

/// The size of an array's header: the type, the identity hash, the length,
/// and 4 bytes that put the elements of a `long[]` on an 8-byte boundary.
pub const ARRAY_HEADER: u32 = 16;
// @end

impl Heap {
    // @listing new-array
    /// Allocates an array of `length` elements of type `element`, all zero.
    pub fn new_array(&mut self, element: &FieldType, length: u32) -> Result<Ref> {
        let index = match self.element_types.iter().position(|t| t == element) {
            Some(index) => index,
            None => {
                self.element_types.push(element.clone());
                self.element_types.len() - 1
            }
        };
        let size =
            u64::from(ARRAY_HEADER) + u64::from(length) * u64::from(size_of(element));
        let at = self.bytes.len();
        let end = at as u64 + size.next_multiple_of(8);
        if end > u64::from(u32::MAX) {
            let class = "java/lang/OutOfMemoryError";
            return Err(Error::Exception {
                class,
                message: Some("Java heap space".into()),
            });
        }
        self.bytes.resize(end as usize, 0);
        let r = Ref(at as u32);
        self.put_u32(at, ARRAY | index as u32);
        self.put_u32(at + 4, r.0);
        self.put_u32(at + 8, length);
        Ok(r)
    }

    /// Whether the object at `r` is an array.
    pub fn is_array(&self, r: Ref) -> bool {
        self.u32_at(r.0 as usize) & ARRAY != 0
    }

    /// The length of the array at `r`.
    pub fn array_length(&self, r: Ref) -> u32 {
        self.u32_at(r.0 as usize + 8)
    }

    /// The type of the elements of the array at `r`.
    pub fn element_type(&self, r: Ref) -> &FieldType {
        &self.element_types[(self.u32_at(r.0 as usize) & !ARRAY) as usize]
    }

    /// The offset of element `index` of the array at `r`, if it has one.
    pub fn element(&self, r: Ref, index: i32) -> Result<u32> {
        let length = self.array_length(r);
        match u32::try_from(index) {
            Ok(i) if i < length => {
                Ok(r.0 + ARRAY_HEADER + i * size_of(self.element_type(r)))
            }
            _ => {
                let m = format!("Index {index} out of bounds for length {length}");
                let class = "java/lang/ArrayIndexOutOfBoundsException";
                Err(Error::Exception { class, message: Some(m) })
            }
        }
    }
    // @end
}
