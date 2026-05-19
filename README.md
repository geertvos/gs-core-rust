# gs-core

A stack-based virtual machine for dynamic object-oriented scripting languages, written in Rust.

## Overview

gs-core (GVM - Geert Virtual Machine) is a bytecode interpreter that provides the runtime foundation for building scripting languages. It executes a compact binary instruction set and includes support for:

- **Stack-based execution** with a full instruction set (arithmetic, logic, control flow, object operations)
- **Dynamic typing** via a pluggable type system (`Type` trait) with runtime dispatch
- **Heap-allocated objects** with a mark-and-sweep garbage collector
- **Multi-threading** with a fork-based concurrency model
- **Native bridge** for extending the VM with Rust modules (`NativeModule` / `NativeInstance` traits)
- **Exception handling** with try/catch semantics and stack unwinding
- **Binary serialization** for loading and storing compiled programs

## Architecture

```
src/
  core/       VM engine: fetch-decode-execute loop, type system, objects, threads
  program/    Program representation: functions, heap, string constants, serializer
  bridge/     Native module interface and method registry
  gc/         Mark-and-sweep garbage collector
  streams/    Random-access byte stream for bytecode
  debug/      Debug utilities
```

## Usage

gs-core is designed as a library. A language implementation (e.g. [gs-lang-rust](https://github.com/geertvos/gs-lang-rust)) compiles source code into GVM bytecode and registers language-specific types and native modules.

```rust
use gs_core::program::{GvmProgram, GvmProgramSerializer};
use gs_core::core::gvm::Gvm;

// Load a compiled program
let program = GvmProgramSerializer::read_from(&mut input, &factory)?;

// Run it
let mut vm = Gvm::new(program);
vm.run();
```

### Extending with native modules

Implement the `NativeModule` trait to expose Rust functionality to scripts:

```rust
use gs_core::bridge::native_module::*;

struct MyModule;

impl NativeModule for MyModule {
    fn class_name(&self) -> &str { "MyModule" }
    fn constructor(&self, args: Vec<NativeValue>) -> NativeResult { /* ... */ }
    fn call_static(&self, method: &str, args: Vec<NativeValue>) -> NativeResult { /* ... */ }
    fn static_methods(&self) -> Vec<MethodDescriptor> { /* ... */ }
}
```

## Instruction Set

| Opcode | Name       | Description                          |
|--------|------------|--------------------------------------|
| 1      | NEW        | Create a new object by type name     |
| 2      | LDS        | Load from stack (relative)           |
| 8      | INVOKE     | Call a function                       |
| 9      | RETURN     | Return from function                  |
| 10     | PUT        | Store value to stack/heap location   |
| 11     | GET        | Get field from object                |
| 12     | HALT       | Stop execution                       |
| 14-17  | ADD/SUB/MULT/DIV | Arithmetic operations          |
| 18-20  | AND/OR/NOT | Logical operations                   |
| 21-23  | EQL/GT/LT  | Comparison operations                |
| 24     | CJMP       | Conditional jump                     |
| 25     | JMP        | Unconditional jump                   |
| 27     | POP        | Discard top of stack                 |
| 28     | NATIVE     | Call native method                   |
| 29     | DUP        | Duplicate top of stack               |
| 30     | MOD        | Modulo                               |
| 31     | THROW      | Throw exception                      |
| 34     | LDC_D      | Load typed constant                  |
| 35     | GETDYNAMIC | Dynamic scope lookup                 |
| 37     | FORK       | Fork execution thread                |

## Building

```sh
cargo build
```

## License

Copyright (c) Geert Vos. All rights reserved.
