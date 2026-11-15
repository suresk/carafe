//! Chapter 12: arrays.

use crate::heap::{ARRAY_HEADER, Heap, Ref, size_of};

// @listing walk
/// Every object and array in the heap, in the order they were allocated:
/// Challenge 7.3's walk, which now reads an array's size from its header.
pub fn objects_and_arrays(heap: &Heap) -> Vec<Ref> {
    let mut found = Vec::new();
    let mut at = 8;
    while (at as usize) < heap.bytes().len() {
        let r = Ref(at);
        found.push(r);
        let size = match heap.is_array(r) {
            true => {
                ARRAY_HEADER + heap.array_length(r) * size_of(heap.element_type(r))
            }
            false => heap.class_of(r).layout.size,
        };
        at += size.next_multiple_of(8);
    }
    found
}
// @end
