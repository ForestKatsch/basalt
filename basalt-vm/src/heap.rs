// Basalt Heap - Reference counting for heap-allocated objects.
// Since we use Arc<RefCell<HeapObject>>, garbage collection is handled
// by Rust's reference counting. No manual GC needed.
