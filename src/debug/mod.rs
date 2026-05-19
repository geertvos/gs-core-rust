use std::io::Write;

use crate::core::gvm::*;
use crate::program::GvmProgram;
use crate::streams::RandomAccessByteStream;

pub struct DebugInfo;

impl DebugInfo {
    pub fn display_program(p: &GvmProgram) {
        for (key, function) in p.functions() {
            println!("Function: {}", key);
            println!("Parameters:");
            println!("{:?}", function.parameters());
            println!("Locals:");
            println!("{:?}", function.locals());
            println!("Bytecode:");
            Self::display_function(&mut std::io::stdout(), &mut function.bytecode().clone());
            println!();
        }
    }

    pub fn disassemble(out: &mut dyn Write, program: &GvmProgram) {
        let strings = program.string_constants();

        let resolve = |idx: i32| -> String {
            if let Some(s) = strings.get(idx as usize) {
                format!("{:?}", s)
            } else {
                format!("str#{}", idx)
            }
        };

        let _ = writeln!(out, "; program: {}", program.name());
        let _ = writeln!(out, "; strings: {}", strings.len());
        let _ = writeln!(out, "; functions: {}", program.functions().len());
        let _ = writeln!(out);

        let _ = writeln!(out, ".strings");
        for (i, s) in strings.iter().enumerate() {
            let _ = writeln!(out, "  {:4}  {:?}", i, s);
        }
        let _ = writeln!(out);

        let mut func_ids: Vec<i32> = program.functions().keys().copied().collect();
        func_ids.sort();

        for func_id in func_ids {
            let function = program.get_function(func_id).unwrap();
            let _ = write!(out, ".function {}", func_id);
            if let Some(name) = function.debug_name() {
                let _ = write!(out, " ; {}", name);
            }
            if !function.parameters().is_empty() {
                let _ = write!(out, "({})", function.parameters().join(", "));
            }
            let _ = writeln!(out);
            if !function.locals().is_empty() {
                let _ = writeln!(out, "  .locals {}", function.locals().join(", "));
            }
            let handlers = function.get_exception_handlers();
            for h in &handlers {
                let _ = writeln!(out, "  .catch {}..{} -> {}", h[0], h[1], h[2]);
            }

            let mut bc = function.bytecode().clone();
            bc.seek(0);
            while bc.get_pointer_position() < bc.size() {
                let addr = bc.get_pointer_position();
                let instruction = bc.read_byte();
                let _ = write!(out, "  {:5}  ", addr);
                match instruction {
                    NEW => {
                        let type_name = bc.read_string();
                        let _ = writeln!(out, "NEW          {}", type_name);
                    }
                    LDS => {
                        let arg = bc.read_int();
                        let _ = writeln!(out, "LDS          {}", arg);
                    }
                    DUP => {
                        let _ = writeln!(out, "DUP");
                    }
                    LDC_D => {
                        let val = bc.read_int();
                        let type_name = bc.read_string();
                        match type_name.as_str() {
                            "String" => {
                                let _ = writeln!(out, "LDC          {} ; {}", resolve(val), type_name);
                            }
                            "Number" => {
                                let _ = writeln!(out, "LDC          {} ; {}", val, type_name);
                            }
                            "Boolean" => {
                                let _ = writeln!(out, "LDC          {} ; {}", if val != 0 { "true" } else { "false" }, type_name);
                            }
                            _ => {
                                let _ = writeln!(out, "LDC          {} ; {}", val, type_name);
                            }
                        }
                    }
                    INVOKE => {
                        let arg = bc.read_int();
                        let _ = writeln!(out, "INVOKE       {}", arg);
                    }
                    RETURN => {
                        let _ = writeln!(out, "RETURN");
                    }
                    PUT => {
                        let _ = writeln!(out, "PUT");
                    }
                    GET => {
                        let _ = writeln!(out, "GET");
                    }
                    HALT => {
                        let _ = writeln!(out, "HALT");
                    }
                    ADD => { let _ = writeln!(out, "ADD"); }
                    SUB => { let _ = writeln!(out, "SUB"); }
                    MULT => { let _ = writeln!(out, "MULT"); }
                    DIV => { let _ = writeln!(out, "DIV"); }
                    MOD => { let _ = writeln!(out, "MOD"); }
                    AND => { let _ = writeln!(out, "AND"); }
                    OR => { let _ = writeln!(out, "OR"); }
                    NOT => { let _ = writeln!(out, "NOT"); }
                    EQL => { let _ = writeln!(out, "EQL"); }
                    LT => { let _ = writeln!(out, "LT"); }
                    GT => { let _ = writeln!(out, "GT"); }
                    JMP => {
                        let pc = bc.read_int();
                        let _ = writeln!(out, "JMP          @{}", pc);
                    }
                    CJMP => {
                        let pc = bc.read_int();
                        let _ = writeln!(out, "CJMP         @{}", pc);
                    }
                    POP => { let _ = writeln!(out, "POP"); }
                    NATIVE => { let _ = writeln!(out, "NATIVE"); }
                    THROW => { let _ = writeln!(out, "THROW"); }
                    DEBUG => {
                        let line = bc.read_int();
                        let loc = bc.read_int();
                        let _ = writeln!(out, "DEBUG        line {} ; {}", line, resolve(loc));
                    }
                    BREAKPOINT => { let _ = writeln!(out, "BREAKPOINT"); }
                    FORK => { let _ = writeln!(out, "FORK"); }
                    GETDYNAMIC => { let _ = writeln!(out, "GETDYNAMIC"); }
                    other => { let _ = writeln!(out, "??? opcode {}", other); }
                }
            }
            let _ = writeln!(out);
        }
    }

    pub fn display_function(out: &mut dyn Write, bytecode: &mut RandomAccessByteStream) {
        bytecode.seek(0);
        while bytecode.get_pointer_position() < bytecode.size() {
            let instruction = bytecode.read_byte();
            match instruction {
                NEW => {
                    let type_name = bytecode.read_string();
                    let _ = writeln!(out, "NEW {}", type_name);
                }
                LDS => {
                    let arg = bytecode.read_int();
                    let _ = writeln!(out, "LDS {}", arg);
                }
                DUP => {
                    let _ = writeln!(out, "DUP");
                }
                LDC_D => {
                    let arg = bytecode.read_int();
                    let type_name = bytecode.read_string();
                    let _ = writeln!(out, "LDC_D {} {}", arg, type_name);
                }
                INVOKE => {
                    let arg = bytecode.read_int();
                    let _ = writeln!(out, "INVOKE {}", arg);
                }
                RETURN => {
                    let _ = writeln!(out, "RETURN");
                }
                PUT => {
                    let _ = writeln!(out, "PUT");
                }
                GET => {
                    let _ = writeln!(out, "GET");
                }
                HALT => {
                    let _ = writeln!(out, "HALT");
                }
                ADD => { let _ = writeln!(out, "ADD"); }
                SUB => { let _ = writeln!(out, "SUB"); }
                MULT => { let _ = writeln!(out, "MULT"); }
                DIV => { let _ = writeln!(out, "DIV"); }
                MOD => { let _ = writeln!(out, "MOD"); }
                AND => { let _ = writeln!(out, "AND"); }
                OR => { let _ = writeln!(out, "OR"); }
                NOT => { let _ = writeln!(out, "NOT"); }
                EQL => { let _ = writeln!(out, "EQL"); }
                LT => { let _ = writeln!(out, "LT"); }
                GT => { let _ = writeln!(out, "GT"); }
                JMP => {
                    let pc = bytecode.read_int();
                    let _ = writeln!(out, "JMP {}", pc);
                }
                CJMP => {
                    let pc = bytecode.read_int();
                    let _ = writeln!(out, "CJMP {}", pc);
                }
                POP => { let _ = writeln!(out, "POP"); }
                NATIVE => { let _ = writeln!(out, "NATIVE"); }
                THROW => { let _ = writeln!(out, "THROW"); }
                DEBUG => {
                    let line = bytecode.read_int();
                    let loc = bytecode.read_int();
                    let _ = writeln!(out, "DEBUG {} {}", line, loc);
                }
                BREAKPOINT => { let _ = writeln!(out, "BREAKPOINT"); }
                FORK => { let _ = writeln!(out, "FORK"); }
                GETDYNAMIC => { let _ = writeln!(out, "GETDYNAMIC"); }
                _ => {}
            }
        }
    }
}
