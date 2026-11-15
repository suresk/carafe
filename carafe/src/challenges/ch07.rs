//! Chapter 7: an object is bytes.

use crate::class::{Layout, Slot};
use crate::class_file::ClassFile;
use crate::flags::AccessFlags;
use crate::heap::{Heap, Ref, size_of};

// @listing packed-layout
/// A layout that puts each field, widest first, at the lowest aligned
/// offset that is free, so that narrow fields fill the gaps that alignment
/// leaves after the superclass's fields.
pub fn packed_layout(file: &ClassFile, superclass: &Layout) -> Layout {
    let mut own: Vec<_> = file.fields.iter().collect();
    own.retain(|f| !f.access_flags.has(AccessFlags::STATIC));
    own.sort_by_key(|f| std::cmp::Reverse(size_of(&f.ty)));
    let mut fields = superclass.fields.clone();
    let mut used: Vec<(u32, u32)> = vec![(0, superclass.size)]; // [start, end)
    for field in own {
        let size = size_of(&field.ty);
        let mut offset = 0;
        while used.iter().any(|&(start, end)| offset < end && start < offset + size) {
            offset += size;
        }
        used.push((offset, offset + size));
        let (name, descriptor) = (field.name.clone(), field.descriptor.clone());
        fields.push(Slot { name, descriptor, ty: field.ty.clone(), offset });
    }
    let size = used.iter().map(|&(_, end)| end).max().unwrap_or(0);
    fields.sort_by_key(|f| f.offset);
    Layout { size, fields }
}
// @end

// @listing objects
/// Every object in the heap, in the order they were allocated, found by
/// walking from one object to the next by their sizes.
pub fn objects(heap: &Heap) -> Vec<Ref> {
    let mut found = Vec::new();
    let mut at = 8; // the first 8 bytes are never an object
    while (at as usize) < heap.bytes().len() {
        let r = Ref(at);
        found.push(r);
        at += heap.class_of(r).layout.size.next_multiple_of(8);
    }
    found
}
// @end
