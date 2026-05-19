use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::bridge::{NativeMethodWrapper, ValueConverter};
use crate::core::exception::GvmExceptionHandler;
use crate::core::types::{BooleanType, FunctionType, Type, UndefinedType};

use super::function::GvmFunction;

pub struct GvmProgram {
    name: String,
    functions: HashMap<i32, GvmFunction>,
    string_constants: Vec<String>,
    types: HashMap<String, Box<dyn Type>>,
    exception_handler: Box<dyn GvmExceptionHandler>,
    converter: Box<dyn ValueConverter>,
    native_wrappers: Vec<Box<dyn NativeMethodWrapper>>,
    function_counter: AtomicI32,
}

impl GvmProgram {
    pub fn new(
        name: String,
        exception_handler: Box<dyn GvmExceptionHandler>,
        converter: Box<dyn ValueConverter>,
    ) -> Self {
        let mut program = GvmProgram {
            name,
            functions: HashMap::new(),
            string_constants: Vec::new(),
            types: HashMap::new(),
            exception_handler,
            converter,
            native_wrappers: Vec::new(),
            function_counter: AtomicI32::new(0),
        };
        program.register_type(Box::new(BooleanType));
        program.register_type(Box::new(UndefinedType));
        program.register_type(Box::new(FunctionType));
        program
    }

    pub fn add_string_at(&mut self, s: String, index: usize) {
        self.string_constants.insert(index, s);
    }

    pub fn add_string(&mut self, s: String) -> i32 {
        if let Some(pos) = self.string_constants.iter().position(|x| x == &s) {
            return pos as i32;
        }
        self.string_constants.push(s);
        (self.string_constants.len() - 1) as i32
    }

    pub fn find_string(&self, s: &str) -> i32 {
        self.string_constants
            .iter()
            .position(|x| x == s)
            .map(|p| p as i32)
            .unwrap_or(-1)
    }

    pub fn get_string(&self, i: i32) -> Option<&str> {
        self.string_constants.get(i as usize).map(|s| s.as_str())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_main(&self) -> Option<&GvmFunction> {
        self.functions.get(&0)
    }

    pub fn get_function(&self, i: i32) -> Option<&GvmFunction> {
        self.functions.get(&i)
    }

    pub fn get_function_mut(&mut self, i: i32) -> Option<&mut GvmFunction> {
        self.functions.get_mut(&i)
    }

    pub fn native_wrappers(&self) -> &[Box<dyn NativeMethodWrapper>] {
        &self.native_wrappers
    }

    pub fn native_wrappers_mut(&mut self) -> &mut Vec<Box<dyn NativeMethodWrapper>> {
        &mut self.native_wrappers
    }

    pub fn add_native(&mut self, method: Box<dyn NativeMethodWrapper>) -> i32 {
        self.native_wrappers.push(method);
        (self.native_wrappers.len() - 1) as i32
    }

    pub fn set_natives(&mut self, natives: Vec<Box<dyn NativeMethodWrapper>>) {
        self.native_wrappers = natives;
    }

    pub fn string_constants(&self) -> &[String] {
        &self.string_constants
    }

    pub fn add_function(&mut self, function: GvmFunction) -> i32 {
        let id = self.function_counter.fetch_add(1, Ordering::SeqCst);
        self.functions.insert(id, function);
        id
    }

    pub fn add_function_with_id(&mut self, id: i32, function: GvmFunction) {
        self.functions.insert(id, function);
        let next = id + 1;
        if next > self.function_counter.load(Ordering::SeqCst) {
            self.function_counter.store(next, Ordering::SeqCst);
        }
    }

    pub fn delete_function(&mut self, id: i32) {
        self.functions.remove(&id);
    }

    pub fn functions(&self) -> &HashMap<i32, GvmFunction> {
        &self.functions
    }

    pub fn get_type(&self, type_name: &str) -> &dyn Type {
        self.types
            .get(type_name)
            .unwrap_or_else(|| panic!("Type: {} is not a known type.", type_name))
            .as_ref()
    }

    pub fn register_type(&mut self, type_: Box<dyn Type>) {
        let name = type_.name().to_string();
        self.types.insert(name, type_);
    }

    pub fn exception_handler(&self) -> &dyn GvmExceptionHandler {
        self.exception_handler.as_ref()
    }

    pub fn take_exception_handler(&mut self) -> Box<dyn GvmExceptionHandler> {
        std::mem::replace(
            &mut self.exception_handler,
            Box::new(crate::core::exception::NoOpExceptionHandler),
        )
    }

    pub fn restore_exception_handler(&mut self, handler: Box<dyn GvmExceptionHandler>) {
        self.exception_handler = handler;
    }

    pub fn converter(&self) -> &dyn ValueConverter {
        self.converter.as_ref()
    }
}
