use std::collections::HashMap;
use std::io::{Read, Write};

use crate::bridge::{NativeMethodWrapper, ValueConverter};
use crate::core::exception::GvmExceptionHandler;
use crate::program::function::GvmFunction;
use crate::program::program::GvmProgram;
use crate::streams::RandomAccessByteStream;

const MAGIC: &[u8; 4] = b"GSVM";
const VERSION: i32 = 2;

pub trait NativeMethodFactory: Send + Sync {
    fn create(&self, argument_count: i32) -> Box<dyn NativeMethodWrapper>;
}

struct StringTable {
    strings: Vec<String>,
    index: HashMap<String, i32>,
}

impl StringTable {
    fn new() -> Self {
        StringTable {
            strings: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn intern(&mut self, s: &str) -> i32 {
        if let Some(&idx) = self.index.get(s) {
            return idx;
        }
        let idx = self.strings.len() as i32;
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), idx);
        idx
    }
}

pub struct GvmProgramSerializer;

impl GvmProgramSerializer {
    pub fn write_to(program: &mut GvmProgram, out: &mut dyn Write) -> std::io::Result<()> {
        let mut table = StringTable::new();

        table.intern(program.name());
        for s in program.string_constants() {
            table.intern(s);
        }
        let func_ids: Vec<i32> = program.functions().keys().copied().collect();
        for id in &func_ids {
            let func = program.get_function(*id).unwrap();
            if let Some(name) = func.debug_name() {
                table.intern(name);
            }
            for p in func.parameters() {
                table.intern(p);
            }
            for l in func.locals() {
                table.intern(l);
            }
        }

        let mut stream = RandomAccessByteStream::new();

        stream.write_bytes(MAGIC);
        stream.write_int(VERSION);

        stream.write_int(table.strings.len() as i32);
        for s in &table.strings {
            stream.write_string(s);
        }

        stream.write_int(table.intern(program.name()));

        let string_constants = program.string_constants().to_vec();
        stream.write_int(string_constants.len() as i32);
        for s in &string_constants {
            stream.write_int(table.intern(s));
        }

        let natives = program.native_wrappers();
        stream.write_int(natives.len() as i32);
        for n in natives {
            stream.write_int(n.argument_count());
        }

        stream.write_int(func_ids.len() as i32);
        for id in &func_ids {
            let func = program.get_function(*id).unwrap();
            stream.write_int(*id);
            stream.write_int(func.index());

            match func.debug_name() {
                Some(name) => stream.write_int(table.intern(name)),
                None => stream.write_int(-1),
            }

            let params: Vec<String> = func.parameters().to_vec();
            stream.write_int(params.len() as i32);
            for p in &params {
                stream.write_int(table.intern(p));
            }

            let locals: Vec<String> = func.locals().to_vec();
            stream.write_int(locals.len() as i32);
            for l in &locals {
                stream.write_int(table.intern(l));
            }

            let handlers = func.get_exception_handlers();
            stream.write_int(handlers.len() as i32);
            for h in &handlers {
                stream.write_int(h[0]);
                stream.write_int(h[1]);
                stream.write_int(h[2]);
            }

            let mut bc = func.bytecode().clone();
            let bc_bytes = bc.get_bytes();
            stream.write_int(bc_bytes.len() as i32);
            stream.write_bytes(&bc_bytes);
        }

        stream.write_to(out)
    }

    pub fn read_from(
        input: &mut dyn Read,
        exception_handler: Box<dyn GvmExceptionHandler>,
        converter: Box<dyn ValueConverter>,
        native_factory: &dyn NativeMethodFactory,
    ) -> std::io::Result<GvmProgram> {
        let mut raw = RandomAccessByteStream::new();
        raw.read_from(input)?;

        let magic = raw.read_bytes(4);
        if &magic != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic: expected GSVM",
            ));
        }

        let version = raw.read_int();
        if version != VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported version: {} (expected {})", version, VERSION),
            ));
        }
        Self::read_v2(&mut raw, exception_handler, converter, native_factory)
    }

    fn read_v2(
        raw: &mut RandomAccessByteStream,
        exception_handler: Box<dyn GvmExceptionHandler>,
        converter: Box<dyn ValueConverter>,
        native_factory: &dyn NativeMethodFactory,
    ) -> std::io::Result<GvmProgram> {
        let table_size = raw.read_int();
        let mut string_table = Vec::with_capacity(table_size as usize);
        for _ in 0..table_size {
            string_table.push(raw.read_string());
        }

        let name_idx = raw.read_int() as usize;
        let name = string_table[name_idx].clone();
        let mut program = GvmProgram::new(name, exception_handler, converter);

        let string_count = raw.read_int();
        for i in 0..string_count {
            let idx = raw.read_int() as usize;
            program.add_string_at(string_table[idx].clone(), i as usize);
        }

        let native_count = raw.read_int();
        for _ in 0..native_count {
            let arg_count = raw.read_int();
            program.add_native(native_factory.create(arg_count));
        }

        let function_count = raw.read_int();
        for _ in 0..function_count {
            let id = raw.read_int();
            let index = raw.read_int();

            let debug_name_idx = raw.read_int();
            let debug_name = if debug_name_idx >= 0 {
                Some(string_table[debug_name_idx as usize].clone())
            } else {
                None
            };

            let param_count = raw.read_int();
            let mut params = Vec::with_capacity(param_count as usize);
            for _ in 0..param_count {
                let idx = raw.read_int() as usize;
                params.push(string_table[idx].clone());
            }

            let local_count = raw.read_int();
            let mut locals = Vec::with_capacity(local_count as usize);
            for _ in 0..local_count {
                let idx = raw.read_int() as usize;
                locals.push(string_table[idx].clone());
            }

            let handler_count = raw.read_int();
            let mut handlers = Vec::with_capacity(handler_count as usize);
            for _ in 0..handler_count {
                handlers.push([raw.read_int(), raw.read_int(), raw.read_int()]);
            }

            let bc_size = raw.read_int() as usize;
            let bc_bytes = raw.read_bytes(bc_size);
            let mut bytecode = RandomAccessByteStream::new();
            bytecode.write_bytes(&bc_bytes);
            bytecode.seek(0);

            let mut func = GvmFunction::new(bytecode, params);
            func.set_index(index);
            if let Some(name) = debug_name {
                func.set_debug_name(name);
            }
            for l in locals {
                func.register_local_variable(l);
            }
            for h in handlers {
                func.register_catch_block(h[0], h[1], h[2]);
            }
            program.add_function_with_id(id, func);
        }

        Ok(program)
    }

}
