pub mod context;
pub mod function;
pub mod heap;
pub mod program;
pub mod serializer;

pub use function::GvmFunction;
pub use heap::GvmHeap;
pub use program::GvmProgram;
pub use serializer::{GvmProgramSerializer, NativeMethodFactory};

// Re-export GvmContext but note it's defined in context module
pub use context::GvmContext;
