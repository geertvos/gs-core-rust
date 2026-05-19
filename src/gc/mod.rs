use std::collections::HashSet;

use crate::core::thread::GvmThread;
use crate::core::types::Operation;
use crate::program::GvmHeap;

pub trait GarbageCollector {
    fn collect(&mut self, heap: &mut GvmHeap, threads: &[GvmThread]);
}

pub struct MarkAndSweepGarbageCollector {
    current_heap_size_threshold: usize,
}

impl MarkAndSweepGarbageCollector {
    pub fn new() -> Self {
        MarkAndSweepGarbageCollector {
            current_heap_size_threshold: 200,
        }
    }

    fn search(&self, obj_ref: i32, alive: &mut HashSet<i32>, heap: &GvmHeap) {
        if alive.contains(&obj_ref) {
            return;
        }
        alive.insert(obj_ref);
        if let Some(obj) = heap.get_object(obj_ref) {
            for v in obj.get_values() {
                if v.type_.supports_operation(Operation::Get) {
                    if heap.get_object(v.value).is_some() {
                        self.search(v.value, alive, heap);
                    }
                }
            }
        }
    }
}

impl Default for MarkAndSweepGarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl GarbageCollector for MarkAndSweepGarbageCollector {
    fn collect(&mut self, heap: &mut GvmHeap, threads: &[GvmThread]) {
        if heap.size() < self.current_heap_size_threshold {
            return;
        }
        let mut alive = HashSet::new();
        for thread in threads {
            for frame in &thread.call_stack {
                let v = &frame.scope;
                if v.type_.supports_operation(Operation::Get) && heap.get_object(v.value).is_some()
                {
                    self.search(v.value, &mut alive, heap);
                }
            }
            for v in &thread.stack {
                if !alive.contains(&v.value)
                    && v.type_.supports_operation(Operation::Get)
                    && heap.get_object(v.value).is_some()
                {
                    self.search(v.value, &mut alive, heap);
                }
            }
        }
        heap.retain_keys(&alive);
        while heap.size() > self.current_heap_size_threshold {
            self.current_heap_size_threshold *= 2;
        }
        eprintln!("Running GC completed");
    }
}
