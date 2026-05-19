use crate::core::stack_frame::StackFrame;
use crate::core::thread::GvmThread;
use crate::core::types::{BooleanType, FunctionType, Operation, Type, TypeContext, UndefinedType, Value, ValueSource};
use crate::gc::{GarbageCollector, MarkAndSweepGarbageCollector};
use crate::program::{GvmHeap, GvmProgram};
use crate::streams::RandomAccessByteStream;

// Instruction constants matching the Java GVM opcodes
pub const NEW: u8 = 1;
pub const LDS: u8 = 2;
pub const INVOKE: u8 = 8;
pub const RETURN: u8 = 9;
pub const PUT: u8 = 10;
pub const GET: u8 = 11;
pub const HALT: u8 = 12;
pub const ADD: u8 = 14;
pub const SUB: u8 = 15;
pub const MULT: u8 = 16;
pub const DIV: u8 = 17;
pub const AND: u8 = 18;
pub const OR: u8 = 19;
pub const NOT: u8 = 20;
pub const EQL: u8 = 21;
pub const GT: u8 = 22;
pub const LT: u8 = 23;
pub const CJMP: u8 = 24;
pub const JMP: u8 = 25;
pub const POP: u8 = 27;
pub const NATIVE: u8 = 28;
pub const DUP: u8 = 29;
pub const MOD: u8 = 30;
pub const THROW: u8 = 31;
pub const DEBUG: u8 = 32;
pub const BREAKPOINT: u8 = 33;
pub const LDC_D: u8 = 34;
pub const GETDYNAMIC: u8 = 35;
pub const FORK: u8 = 37;

pub struct Gvm {
    gc: Box<dyn GarbageCollector>,
    pub heap: GvmHeap,
    pub program: GvmProgram,
    threads: Vec<GvmThread>,
    running: Vec<bool>,
    debug: bool,
}

struct VmTypeContext<'a> {
    heap: &'a mut GvmHeap,
    program: &'a mut GvmProgram,
    native_wrapper_offset: i32,
}

impl TypeContext for VmTypeContext<'_> {
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
        let native_idx = self.program.add_native(wrapper);
        let real_idx = native_idx + self.native_wrapper_offset;
        let mut code = RandomAccessByteStream::new();
        let mut param_names = Vec::new();
        for i in 0..arg_count {
            param_names.push(format!("p{}", i));
            code.write_byte(LDS);
            code.write_int(1 + i);
        }
        code.write_byte(LDC_D);
        code.write_int(real_idx);
        code.write_string("Function");
        code.write_byte(NATIVE);
        code.write_byte(RETURN);
        let function = crate::program::GvmFunction::new(code, param_names);
        self.program.add_function(function)
    }
}

impl Gvm {
    pub fn new(program: GvmProgram) -> Self {
        Self::with_heap(program, GvmHeap::new())
    }

    pub fn with_heap(program: GvmProgram, heap: GvmHeap) -> Self {
        Gvm {
            gc: Box::new(MarkAndSweepGarbageCollector::new()),
            heap,
            program,
            threads: Vec::new(),
            running: Vec::new(),
            debug: false,
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn spawn_thread(&mut self) -> usize {
        let thread = GvmThread::new();
        self.threads.push(thread);
        self.running.push(false);
        self.threads.len() - 1
    }

    pub fn run(&mut self) {
        let main_idx = self.spawn_thread();
        self.running[main_idx] = true;
        self.heap.clear();

        let mut bytecode = RandomAccessByteStream::new();
        bytecode.write_byte(NEW);
        bytecode.write_string("Object");
        bytecode.write_byte(LDC_D);
        bytecode.write_int(0);
        bytecode.write_string(&FunctionType.name().to_string());
        bytecode.write_byte(INVOKE);
        bytecode.write_int(0);
        bytecode.write_byte(HALT);
        bytecode.seek(0);

        self.threads[main_idx].bytecode = bytecode;
        self.fetch_and_decode_all();
        println!("> VM exited normal");
    }

    pub fn inject(&mut self, thread_idx: usize) {
        let no_running = !self.running.iter().any(|r| *r);
        self.running[thread_idx] = true;
        if no_running {
            self.fetch_and_decode_all();
        }
    }

    fn fetch_and_decode_all(&mut self) {
        loop {
            let running_indices: Vec<usize> = self
                .running
                .iter()
                .enumerate()
                .filter(|(_, r)| **r)
                .map(|(i, _)| i)
                .collect();
            if running_indices.is_empty() {
                break;
            }
            for idx in running_indices {
                let continues = self.fetch_and_decode(idx);
                if !continues {
                    self.running[idx] = false;
                }
            }
        }
    }

    pub fn fetch_and_decode(&mut self, thread_idx: usize) -> bool {
        let pc = if self.debug {
            Some(self.threads[thread_idx].bytecode.get_pointer_position())
        } else {
            None
        };
        let instruction = self.threads[thread_idx].bytecode.read_byte();
        if let Some(pc) = pc {
            let iname = match instruction {
                NEW => "NEW", LDS => "LDS", INVOKE => "INVOKE", RETURN => "RETURN",
                PUT => "PUT", GET => "GET", HALT => "HALT", ADD => "ADD", SUB => "SUB",
                MULT => "MULT", DIV => "DIV", AND => "AND", OR => "OR", NOT => "NOT",
                EQL => "EQL", GT => "GT", LT => "LT", CJMP => "CJMP", JMP => "JMP",
                POP => "POP", NATIVE => "NATIVE", DUP => "DUP", MOD => "MOD",
                THROW => "THROW", DEBUG => "DEBUG", BREAKPOINT => "BREAKPOINT",
                LDC_D => "LDC_D", GETDYNAMIC => "GETDYNAMIC", FORK => "FORK",
                _ => "???",
            };
            let stk: Vec<String> = self.threads[thread_idx].stack.iter().map(|v| format!("{}:{}", v.value, v.type_.name())).collect();
            eprintln!("[pc={} f={} fp={}] {} | stk={:?}", pc, self.threads[thread_idx].function_pointer, self.threads[thread_idx].frame_pointer, iname, stk);
        }

        match instruction {
            NEW => {
                let type_name = self.threads[thread_idx].bytecode.read_string();
                let type_ = self.program.get_type(&type_name).clone_type();
                if type_.supports_operation(Operation::New) {
                    let dummy = Value::undefined();
                    let mut ctx = VmTypeContext {
                        heap: &mut self.heap,
                        program: &mut self.program,
                        native_wrapper_offset: 0,
                    };
                    let v = type_.perform(&mut ctx, Operation::New, &dummy, None);
                    self.threads[thread_idx].stack_push(v);
                } else {
                    self.handle_exception(
                        thread_idx,
                        &format!("Type {} does not support NEW.", type_name),
                    );
                }
            }
            LDS => {
                let pos = self.threads[thread_idx].bytecode.read_int();
                let thread = &self.threads[thread_idx];
                if pos >= 0 {
                    let arg = (thread.frame_pointer + pos) as usize;
                    let mut val = thread.stack_get(arg);
                    val.source = ValueSource::Stack(arg);
                    self.threads[thread_idx].stack_push(val);
                } else {
                    let idx = (thread.stack_size() as i32 + pos - 1) as usize;
                    let mut val = thread.stack_get(idx);
                    val.source = ValueSource::Stack(idx);
                    self.threads[thread_idx].stack_push(val);
                }
            }
            DUP => {
                let val = self.threads[thread_idx].stack_peek();
                self.threads[thread_idx].stack_push(val);
            }
            LDC_D => {
                let arg = self.threads[thread_idx].bytecode.read_int();
                let type_name = self.threads[thread_idx].bytecode.read_string();
                let type_ = self.program.get_type(&type_name).clone_type();
                self.threads[thread_idx].stack_push(Value::new(arg, type_));
            }
            INVOKE => {
                let arg_count = self.threads[thread_idx].bytecode.read_int();
                let callee_function = self.threads[thread_idx].stack_pop();
                if !callee_function.type_.supports_operation(Operation::Invoke) {
                    self.handle_exception(
                        thread_idx,
                        &format!(
                            "Invoking a type that does not support invocation: {}",
                            callee_function.type_.name()
                        ),
                    );
                    return true;
                }

                let caller_function = self.threads[thread_idx].function_pointer;
                self.threads[thread_idx].function_pointer = callee_function.value;
                let function_desc = self
                    .program
                    .get_function(callee_function.value)
                    .expect("Function not found");
                let param_count = function_desc.parameters().len() as i32;
                if arg_count != param_count {
                    self.handle_exception(
                        thread_idx,
                        &format!(
                            "Argument count for function {} is {}, but {} provided.",
                            callee_function.value, param_count, arg_count
                        ),
                    );
                    return true;
                }

                let mut params = Vec::with_capacity(param_count as usize);
                for _ in 0..param_count {
                    params.push(self.threads[thread_idx].stack_pop());
                }
                params.reverse();

                let this_val = self.threads[thread_idx].stack_peek();

                let thread = &self.threads[thread_idx];
                let frame = StackFrame::new(
                    thread.bytecode.get_pointer_position(),
                    thread.frame_pointer,
                    caller_function,
                    thread.debug_line_number,
                    thread.location,
                    this_val,
                );
                self.threads[thread_idx].call_stack.push(frame);
                self.threads[thread_idx].frame_pointer =
                    self.threads[thread_idx].stack_size() as i32 - 1;

                for (i, p) in params.into_iter().enumerate() {
                    let mut param = p;
                    param.comment = Some(format!("Function parameter {}", i));
                    self.threads[thread_idx].stack_push(param);
                }

                let local_count = self
                    .program
                    .get_function(callee_function.value)
                    .unwrap()
                    .locals()
                    .len();
                for i in 0..local_count {
                    self.threads[thread_idx].stack_push(Value::with_comment(
                        0,
                        Box::new(UndefinedType),
                        format!("Local variable {}", i),
                    ));
                }

                let new_bytecode = self
                    .program
                    .get_function(callee_function.value)
                    .unwrap()
                    .bytecode()
                    .clone();
                self.threads[thread_idx].bytecode = new_bytecode;
                self.threads[thread_idx].bytecode.seek(0);
            }
            RETURN => {
                let v = self.threads[thread_idx].stack_pop();

                let fp = self.threads[thread_idx].function_pointer;
                let function = self.program.get_function(fp).expect("Function not found");
                let local_count = function.locals().len();
                let param_count = function.parameters().len();

                for _ in 0..local_count {
                    self.threads[thread_idx].stack_pop();
                }
                for _ in 0..param_count {
                    self.threads[thread_idx].stack_pop();
                }
                self.threads[thread_idx].stack_pop(); // this

                let frame = self.threads[thread_idx]
                    .call_stack
                    .pop()
                    .expect("Call stack underflow");
                self.threads[thread_idx].debug_line_number = frame.line_number;
                self.threads[thread_idx].function_pointer = frame.calling_function;
                self.threads[thread_idx].frame_pointer = frame.frame_pointer;
                self.threads[thread_idx].location = frame.location;
                let pc = frame.program_counter;

                let new_bytecode = self
                    .program
                    .get_function(self.threads[thread_idx].function_pointer)
                    .expect("Function not found")
                    .bytecode()
                    .clone();
                self.threads[thread_idx].bytecode = new_bytecode;
                self.threads[thread_idx].bytecode.seek(pc);
                self.threads[thread_idx].stack_push(v);
                self.gc.collect(&mut self.heap, &self.threads);
            }
            PUT => {
                let to_set = self.threads[thread_idx].stack_pop();
                let value = self.threads[thread_idx].stack_peek();
                let new_val = Value::new(value.value, value.type_.clone());
                match &to_set.source {
                    ValueSource::Stack(idx) => {
                        let idx = *idx;
                        self.threads[thread_idx].stack_set(idx, new_val);
                    }
                    ValueSource::HeapField(obj_ref, field) => {
                        let obj_ref = *obj_ref;
                        let field = field.clone();
                        if let Some(obj) = self.heap.get_object_mut(obj_ref) {
                            obj.set_value(&field, new_val);
                        }
                    }
                    ValueSource::None => {}
                }
            }
            GET => {
                let variable_name = self.threads[thread_idx].stack_pop();
                let reference = self.threads[thread_idx].stack_pop();
                if !reference.type_.supports_operation(Operation::Get) {
                    self.handle_exception(
                        thread_idx,
                        &format!(
                            "Type does not support get operation: {} pc: {} f:{}",
                            reference,
                            self.threads[thread_idx].bytecode.get_pointer_position(),
                            self.threads[thread_idx].function_pointer
                        ),
                    );
                    return true;
                }
                let obj_ref = reference.value;
                let field_name = if variable_name.type_.name() == "String" {
                    self.program.get_string(variable_name.value).map(|s| s.to_string())
                } else {
                    Some(variable_name.value.to_string())
                };
                let type_ = reference.type_.clone();
                let mut ctx = VmTypeContext {
                    heap: &mut self.heap,
                    program: &mut self.program,
                    native_wrapper_offset: 0,
                };
                let mut value = type_.perform(&mut ctx, Operation::Get, &reference, Some(&variable_name));
                if let Some(fname) = field_name {
                    value.source = ValueSource::HeapField(obj_ref, fname);
                }
                self.threads[thread_idx].stack_push(value);
            }
            GETDYNAMIC => {
                let variable = self.threads[thread_idx].stack_pop();
                let variable_name = self
                    .program
                    .get_string(variable.value)
                    .expect("String not found")
                    .to_string();
                let mut the_value: Option<Value> = None;
                let mut source_scope: Option<i32> = None;
                for frame in &self.threads[thread_idx].call_stack {
                    let scope = &frame.scope;
                    if let Some(obj) = self.heap.get_object(scope.value) {
                        if obj.has_value(&variable_name) {
                            the_value = obj.get_value(&variable_name);
                            source_scope = Some(scope.value);
                            break;
                        }
                    }
                }
                if the_value.is_none() {
                    if let Some(frame) = self.threads[thread_idx].call_stack.last() {
                        source_scope = Some(frame.scope.value);
                        if let Some(obj) = self.heap.get_object(frame.scope.value) {
                            the_value = obj.get_value(&variable_name);
                        }
                    }
                }
                let mut value = the_value.unwrap_or_else(Value::undefined);
                if let Some(scope_ref) = source_scope {
                    value.source = ValueSource::HeapField(scope_ref, variable_name);
                }
                self.threads[thread_idx].stack_push(value);
            }
            HALT => {
                self.threads[thread_idx].mark_finished();
                return false;
            }
            ADD => self.binary_op(thread_idx, Operation::Add, "addition"),
            SUB => self.binary_op(thread_idx, Operation::Sub, "substraction"),
            MULT => self.binary_op(thread_idx, Operation::Mult, "multiplication"),
            DIV => self.binary_op(thread_idx, Operation::Div, "division"),
            MOD => self.binary_op(thread_idx, Operation::Mod, "modulo"),
            AND => self.binary_op(thread_idx, Operation::And, "AND"),
            OR => self.binary_op(thread_idx, Operation::Or, "OR"),
            NOT => {
                let arg1 = self.threads[thread_idx].stack_pop();
                if arg1.type_.supports_operation(Operation::Not) {
                    let type_ = arg1.type_.clone();
                    let mut ctx = VmTypeContext {
                        heap: &mut self.heap,
                        program: &mut self.program,
                        native_wrapper_offset: 0,
                    };
                    let result = type_.perform(&mut ctx, Operation::Not, &arg1, None);
                    self.threads[thread_idx].stack_push(result);
                } else {
                    self.handle_exception(
                        thread_idx,
                        &format!("Type {} does not support NOT.", arg1.type_.name()),
                    );
                }
            }
            EQL => self.binary_op(thread_idx, Operation::Eql, "EQL"),
            LT => self.binary_op(thread_idx, Operation::Lt, "LT"),
            GT => self.binary_op(thread_idx, Operation::Gt, "GT"),
            JMP => {
                let pc = self.threads[thread_idx].bytecode.read_int();
                self.threads[thread_idx].bytecode.seek(pc);
            }
            CJMP => {
                let cond = self.threads[thread_idx].stack_pop();
                let jump = self.threads[thread_idx].bytecode.read_int();
                if cond.value > 0 {
                    self.threads[thread_idx].bytecode.seek(jump);
                }
            }
            POP => {
                self.threads[thread_idx].stack_pop();
            }
            NATIVE => {
                let arg = self.threads[thread_idx].stack_pop();
                if !arg.type_.supports_operation(Operation::Invoke) {
                    self.handle_exception(
                        thread_idx,
                        &format!(
                            "Type: {} does not support invocation.",
                            arg.type_.name()
                        ),
                    );
                    return true;
                }
                let wrapper_idx = arg.value as usize;
                let arg_count = self.program.native_wrappers()[wrapper_idx].argument_count();
                let mut args = Vec::with_capacity(arg_count as usize);
                for _ in 0..arg_count {
                    args.push(self.threads[thread_idx].stack_pop());
                }

                // Take the native wrappers out temporarily to avoid borrow conflict
                let native_wrappers = std::mem::take(self.program.native_wrappers_mut());
                let native_offset = native_wrappers.len() as i32;
                let mut ctx = VmTypeContext {
                    heap: &mut self.heap,
                    program: &mut self.program,
                    native_wrapper_offset: native_offset,
                };
                let call_result = native_wrappers[wrapper_idx].invoke(args, &mut ctx);
                // Merge any wrappers added during invoke (e.g. by generate_native_method_function)
                let mut restored = native_wrappers;
                let newly_added = std::mem::take(self.program.native_wrappers_mut());
                restored.extend(newly_added);
                self.program.set_natives(restored);
                match call_result {
                    Ok(val) => {
                        self.threads[thread_idx].stack_push(val);
                    }
                    Err(msg) => {
                        self.handle_exception(thread_idx, &msg);
                        return true;
                    }
                };
                self.gc.collect(&mut self.heap, &self.threads);
            }
            THROW => {
                let arg = self.threads[thread_idx].stack_pop();
                let line = self.threads[thread_idx].debug_line_number;
                let location = self.threads[thread_idx].location;
                let handler = self.program.take_exception_handler();
                let mut ctx = VmTypeContext {
                    heap: &mut self.heap,
                    program: &mut self.program,
                    native_wrapper_offset: 0,
                };
                let exception = handler.convert_value(&arg, &mut ctx, line, location);
                self.program.restore_exception_handler(handler);
                self.handle_exception_object(thread_idx, exception);
            }
            DEBUG => {
                let line = self.threads[thread_idx].bytecode.read_int();
                self.threads[thread_idx].debug_line_number = line;
                let loc = self.threads[thread_idx].bytecode.read_int();
                self.threads[thread_idx].location = loc;
            }
            BREAKPOINT => {
                println!(
                    "Breakpoint current line: {}",
                    self.threads[thread_idx].debug_line_number
                );
            }
            FORK => {
                let new_thread = self.threads[thread_idx].fork(&mut self.heap);
                self.threads.push(new_thread);
                self.running.push(true);
                // Parent gets 0, child gets 1 (like Unix fork)
                self.threads[thread_idx].stack_push(Value::with_comment(
                    0,
                    Box::new(BooleanType),
                    "Fork return value".to_string(),
                ));
                let last = self.threads.len() - 1;
                self.threads[last].stack_push(Value::with_comment(
                    1,
                    Box::new(BooleanType),
                    "Fork return value".to_string(),
                ));
            }
            _ => {}
        }
        true
    }

    fn binary_op(&mut self, thread_idx: usize, op: Operation, op_name: &str) {
        let arg2 = self.threads[thread_idx].stack_pop();
        let arg1 = self.threads[thread_idx].stack_pop();
        if arg1.type_.supports_operation(op) {
            let type_ = arg1.type_.clone();
            let mut ctx = VmTypeContext {
                heap: &mut self.heap,
                program: &mut self.program,
                native_wrapper_offset: 0,
            };
            let result = type_.perform(&mut ctx, op, &arg1, Some(&arg2));
            self.threads[thread_idx].stack_push(result);
        } else {
            self.handle_exception(
                thread_idx,
                &format!("Type {} does not support {}.", arg1.type_.name(), op_name),
            );
        }
    }

    fn handle_exception(&mut self, thread_idx: usize, message: &str) {
        let line = self.threads[thread_idx].debug_line_number;
        let location = self.threads[thread_idx].location;
        let handler = self.program.take_exception_handler();
        let mut ctx = VmTypeContext {
            heap: &mut self.heap,
            program: &mut self.program,
            native_wrapper_offset: 0,
        };
        let value = handler.convert_message(message, &mut ctx, line, location);
        self.program.restore_exception_handler(handler);
        self.handle_exception_object(thread_idx, value);
    }

    fn handle_exception_object(&mut self, thread_idx: usize, exception: Value) {
        let fp = self.threads[thread_idx].function_pointer;
        let f = self.program.get_function(fp).expect("Function not found");
        let catch_block =
            f.get_exception_handler(self.threads[thread_idx].bytecode.get_pointer_position());

        if catch_block > -1 {
            self.threads[thread_idx].stack_push(exception);
            self.threads[thread_idx].bytecode.seek(catch_block);
        } else if self.threads[thread_idx].peel(&self.program) {
            self.handle_exception_object(thread_idx, exception);
        } else {
            let mut message = "Unhandled unknown exception".to_string();
            if exception.type_.supports_operation(Operation::Get) {
                if let Some(obj) = self.heap.get_object(exception.value) {
                    if let Some(msg_val) = obj.get_value("message") {
                        if let Some(msg) = self.program.get_string(msg_val.value) {
                            if let Some(line_val) = obj.get_value("line") {
                                if let Some(loc_val) = obj.get_value("location") {
                                    if let Some(loc) = self.program.get_string(loc_val.value) {
                                        message = format!(
                                            "Unhandled exception '{}' at {}:{}",
                                            msg, loc, line_val.value
                                        );
                                    } else {
                                        message = format!(
                                            "Unhandled exception '{}' at line {}",
                                            msg, line_val.value
                                        );
                                    }
                                } else {
                                    message = format!(
                                        "Unhandled exception '{}' at line {}",
                                        msg, line_val.value
                                    );
                                }
                            }
                        }
                    }
                }
            }
            eprintln!("{}", message);
            std::process::exit(1);
        }
    }
}
