use super::types::Value;

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub program_counter: i32,
    pub frame_pointer: i32,
    pub calling_function: i32,
    pub line_number: i32,
    pub location: i32,
    pub scope: Value,
}

impl StackFrame {
    pub fn new(
        program_counter: i32,
        frame_pointer: i32,
        calling_function: i32,
        line_number: i32,
        location: i32,
        scope: Value,
    ) -> Self {
        StackFrame {
            program_counter,
            frame_pointer,
            calling_function,
            line_number,
            location,
            scope,
        }
    }
}
