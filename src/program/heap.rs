use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::core::object::GvmObject;

pub struct GvmHeap {
    heap: HashMap<i32, Box<dyn GvmObject>>,
    object_id_counter: AtomicI32,
}

impl GvmHeap {
    pub fn new() -> Self {
        GvmHeap {
            heap: HashMap::new(),
            object_id_counter: AtomicI32::new(-1),
        }
    }

    pub fn add_object(&mut self, object: Box<dyn GvmObject>) -> i32 {
        let id = self.object_id_counter.fetch_add(1, Ordering::SeqCst) + 1;
        self.heap.insert(id, object);
        id
    }

    pub fn get_object(&self, key: i32) -> Option<&dyn GvmObject> {
        self.heap.get(&key).map(|o| &**o)
    }

    pub fn get_object_mut(&mut self, key: i32) -> Option<&mut (dyn GvmObject + 'static)> {
        match self.heap.get_mut(&key) {
            Some(o) => Some(o.as_mut()),
            None => None,
        }
    }

    pub fn clear(&mut self) {
        self.heap.clear();
    }

    pub fn size(&self) -> usize {
        self.heap.len()
    }

    pub fn retain_keys(&mut self, keys: &std::collections::HashSet<i32>) {
        let to_remove: Vec<i32> = self
            .heap
            .keys()
            .filter(|k| !keys.contains(k))
            .copied()
            .collect();
        for key in to_remove {
            if let Some(obj) = self.heap.get(&key) {
                obj.pre_destroy();
            }
            self.heap.remove(&key);
        }
    }
}

impl Default for GvmHeap {
    fn default() -> Self {
        Self::new()
    }
}
