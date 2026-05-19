// GvmContext in the Java version provides access to the GVM, program, heap, and thread.
// In the Rust version, the GVM struct owns all state and passes references directly.
// This module is kept for API compatibility - language implementations may use it
// when they need to bundle references together.

use crate::core::thread::GvmThread;
use crate::core::types::{TypeContext, Value};
use crate::program::{GvmHeap, GvmProgram};

pub struct GvmContext<'a> {
    pub program: &'a mut GvmProgram,
    pub heap: &'a mut GvmHeap,
    pub thread: &'a mut GvmThread,
}

impl<'a> GvmContext<'a> {
    pub fn new(
        program: &'a mut GvmProgram,
        heap: &'a mut GvmHeap,
        thread: &'a mut GvmThread,
    ) -> Self {
        GvmContext {
            program,
            heap,
            thread,
        }
    }
}

impl TypeContext for GvmContext<'_> {
    fn heap_get_object_value(&self, obj_ref: i32, field: &str) -> Option<Value> {
        self.heap.get_object(obj_ref)?.get_value(field)
    }

    fn heap_set_object_value(&mut self, obj_ref: i32, field: &str, value: Value) {
        if let Some(obj) = self.heap.get_object_mut(obj_ref) {
            obj.set_value(field, value);
        }
    }

    fn heap_has_object_value(&self, obj_ref: i32, field: &str) -> bool {
        self.heap
            .get_object(obj_ref)
            .is_some_and(|o| o.has_value(field))
    }

    fn heap_add_object(&mut self) -> i32 {
        panic!("heap_add_object must be implemented by the language-specific object type")
    }

    fn heap_add_object_box(&mut self, object: Box<dyn crate::core::object::GvmObject>) -> i32 {
        self.heap.add_object(object)
    }

    fn heap_get_object_keys(&self, obj_ref: i32) -> Vec<String> {
        self.heap.get_object(obj_ref).map(|o| o.get_keys()).unwrap_or_default()
    }

    fn heap_get_object_any(&self, obj_ref: i32) -> Option<&dyn std::any::Any> {
        self.heap.get_object(obj_ref).map(|o| o.as_any())
    }

    fn get_string(&self, index: i32) -> Option<&str> {
        self.program.get_string(index)
    }

    fn add_string(&mut self, s: &str) -> i32 {
        self.program.add_string(s.to_string())
    }

    fn generate_native_method_function(
        &mut self,
        wrapper: Box<dyn crate::bridge::NativeMethodWrapper>,
        arg_count: i32,
    ) -> i32 {
        use crate::core::gvm::*;
        use crate::streams::RandomAccessByteStream;

        let native_idx = self.program.add_native(wrapper);
        let mut code = RandomAccessByteStream::new();
        let mut param_names = Vec::new();
        for i in 0..arg_count {
            param_names.push(format!("p{}", i));
            code.write_byte(LDS);
            code.write_int(1 + i);
        }
        code.write_byte(LDC_D);
        code.write_int(native_idx);
        code.write_string("Function");
        code.write_byte(NATIVE);
        code.write_byte(RETURN);
        let function = crate::program::GvmFunction::new(code, param_names);
        self.program.add_function(function)
    }
}
