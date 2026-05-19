use crate::core::stack_frame::StackFrame;
use crate::core::types::Value;
use crate::program::GvmHeap;
use crate::program::GvmProgram;
use crate::streams::RandomAccessByteStream;

pub struct GvmThread {
    pub frame_pointer: i32,
    pub function_pointer: i32,
    pub debug_line_number: i32,
    pub location: i32,
    pub call_stack: Vec<StackFrame>,
    pub stack: Vec<Value>,
    pub bytecode: RandomAccessByteStream,
    pub finished: bool,
}

impl GvmThread {
    pub fn new() -> Self {
        GvmThread {
            frame_pointer: 0,
            function_pointer: 0,
            debug_line_number: -1,
            location: 0,
            call_stack: Vec::new(),
            stack: Vec::new(),
            bytecode: RandomAccessByteStream::new(),
            finished: false,
        }
    }

    pub fn mark_finished(&mut self) {
        self.finished = true;
    }

    pub fn stack_push(&mut self, v: Value) {
        self.stack.push(v);
    }

    pub fn stack_pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

    pub fn stack_peek(&self) -> Value {
        self.stack.last().expect("Stack underflow").clone()
    }

    pub fn stack_get(&self, index: usize) -> Value {
        self.stack[index].clone()
    }

    pub fn stack_set(&mut self, index: usize, value: Value) {
        self.stack[index] = value;
    }

    pub fn stack_size(&self) -> usize {
        self.stack.len()
    }

    pub fn peel(&mut self, program: &GvmProgram) -> bool {
        if self.call_stack.len() == 1 {
            return false;
        }
        while self.call_stack.len() as i32 > self.frame_pointer {
            self.call_stack.pop();
        }
        let frame = self.call_stack.pop().expect("Call stack underflow during peel");
        self.debug_line_number = frame.line_number;
        self.function_pointer = frame.calling_function;
        self.frame_pointer = frame.frame_pointer;
        self.location = frame.location;
        let pc = frame.program_counter;
        self.bytecode = program
            .get_function(self.function_pointer)
            .expect("Function not found during peel")
            .bytecode()
            .clone();
        self.bytecode.seek(pc);
        true
    }

    pub fn fork(&self, heap: &mut GvmHeap) -> GvmThread {
        let mut new_call_stack = self.call_stack.clone();
        let toclone = new_call_stack.pop().expect("Call stack empty during fork");
        let v = &toclone.scope;
        let cloned_scope = heap
            .get_object(v.value)
            .expect("Scope object not found during fork")
            .clone_object();
        let ref_ = heap.add_object(cloned_scope);
        let new_scope = Value::new(ref_, v.type_.clone());
        new_call_stack.push(StackFrame::new(
            toclone.program_counter,
            toclone.frame_pointer,
            toclone.calling_function,
            toclone.line_number,
            toclone.location,
            new_scope,
        ));

        let new_stack = self.stack.clone();
        let mut thread = GvmThread {
            frame_pointer: self.frame_pointer,
            function_pointer: self.function_pointer,
            debug_line_number: self.debug_line_number,
            location: self.location,
            call_stack: new_call_stack,
            stack: new_stack,
            bytecode: self.bytecode.clone(),
            finished: false,
        };
        thread.bytecode.seek(self.bytecode.get_pointer_position());
        thread
    }
}

impl Default for GvmThread {
    fn default() -> Self {
        Self::new()
    }
}
