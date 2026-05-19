use crate::streams::RandomAccessByteStream;

pub(crate) struct ExceptionHandler {
    pub(crate) try_start: i32,
    pub(crate) try_end: i32,
    pub(crate) catch_start: i32,
}

pub struct GvmFunction {
    bytecode: RandomAccessByteStream,
    parameters: Vec<String>,
    locals: Vec<String>,
    exception_handlers: Vec<ExceptionHandler>,
    index: i32,
    debug_name: Option<String>,
}

impl GvmFunction {
    pub fn new(bytecode: RandomAccessByteStream, parameters: Vec<String>) -> Self {
        GvmFunction {
            bytecode,
            parameters,
            locals: Vec::new(),
            exception_handlers: Vec::new(),
            index: 0,
            debug_name: None,
        }
    }

    pub fn debug_name(&self) -> Option<&str> {
        self.debug_name.as_deref()
    }

    pub fn set_debug_name(&mut self, name: String) {
        self.debug_name = Some(name);
    }

    pub fn bytecode(&self) -> &RandomAccessByteStream {
        &self.bytecode
    }

    pub fn bytecode_mut(&mut self) -> &mut RandomAccessByteStream {
        &mut self.bytecode
    }

    pub fn set_bytecode(&mut self, bytecode: RandomAccessByteStream) {
        self.bytecode = bytecode;
    }

    pub fn get_exception_handler(&self, program_counter: i32) -> i32 {
        let mut catch_block = -1i32;
        let mut distance = i32::MAX;
        for e in &self.exception_handlers {
            let ld = program_counter - e.try_start;
            if ld > 0 && ld < distance && e.try_end >= program_counter {
                distance = ld;
                catch_block = e.catch_start;
            }
        }
        catch_block
    }

    pub fn register_local_variable(&mut self, name: String) {
        if !self.locals.contains(&name) {
            self.locals.push(name);
        }
    }

    pub fn locals(&self) -> &[String] {
        &self.locals
    }

    pub fn register_catch_block(&mut self, start: i32, end: i32, start_of_catch: i32) {
        self.exception_handlers.push(ExceptionHandler {
            try_start: start,
            try_end: end,
            catch_start: start_of_catch,
        });
    }

    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    pub fn set_index(&mut self, index: i32) {
        self.index = index;
    }

    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn get_exception_handlers(&self) -> Vec<[i32; 3]> {
        self.exception_handlers
            .iter()
            .map(|e| [e.try_start, e.try_end, e.catch_start])
            .collect()
    }
}
