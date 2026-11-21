//! The resolution cache: what each constant pool entry resolved to.

use crate::class::{Class, Slot, State, coerce};
use crate::descriptor::FieldType;
use crate::error::Result;
use crate::interpreter::{Vm, non_null};
use crate::opcode::{GETFIELD, GETSTATIC};
use std::rc::Rc;

// @listing resolved
/// What a constant pool entry resolved to, kept so that it is resolved once.
pub enum Resolved {
    /// A class that `new` may instantiate, initialized.
    Class(Rc<Class>),
    /// A method: the class that declares it, and its index there.
    Method(Rc<Class>, usize),
    /// An instance field: its place in an object.
    Field(Slot),
    /// A static field of an initialized class: the class, the field's index
    /// in its statics, and its type.
    Static(Rc<Class>, usize, FieldType),
    /// A `final` instance field, which only `getfield` takes from the cache,
    /// so that every `putfield` of it is checked.
    FinalField(Slot),
    /// A `final` static field, which only `getstatic` takes from the cache.
    FinalStatic(Rc<Class>, usize, FieldType),
}

impl Class {
    /// What entry `index` of the constant pool resolved to, if it has been.
    pub fn cached(&self, index: u16) -> Option<&Resolved> {
        self.resolved.get(usize::from(index))?.get()
    }

    /// Keeps what entry `index` resolved to. Resolving it again would give
    /// the same answer, so the first one stays.
    pub(crate) fn cache(&self, index: u16, resolved: Resolved) {
        if let Some(cell) = self.resolved.get(usize::from(index)) {
            let _ = cell.set(resolved);
        }
    }
}
// @end

impl Vm {
    // @listing cached-access
    /// `getfield` or `putfield` of a field whose place is known.
    pub(crate) fn field_access(&mut self, opcode: u8, slot: &Slot) -> Result<()> {
        let frame = self.frames.last_mut().expect("a frame");
        if opcode == GETFIELD {
            let object = non_null(frame.pop_ref()?)?;
            frame.push(self.heap.read(object.0 + slot.offset, &slot.ty))
        } else {
            let value = frame.pop()?;
            let object = non_null(frame.pop_ref()?)?;
            self.heap.write(object.0 + slot.offset, &slot.ty, value)
        }
    }

    /// `getstatic` or `putstatic` of a field of an initialized class.
    pub(crate) fn static_access(
        &mut self,
        opcode: u8,
        owner: &Class,
        slot: usize,
        ty: &FieldType,
    ) -> Result<()> {
        let frame = self.frames.last_mut().expect("a frame");
        if opcode == GETSTATIC {
            frame.push(owner.statics.borrow()[slot])
        } else {
            let value = coerce(ty, frame.pop()?)?;
            owner.statics.borrow_mut()[slot] = value;
            Ok(())
        }
    }
    // @end
}

/// Whether `class` is initialized, so that what refers to it can be cached.
pub(crate) fn initialized(class: &Class) -> bool {
    class.state.get() == State::Initialized
}
