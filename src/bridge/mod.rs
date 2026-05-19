pub mod native_module;
pub mod registry;

use std::any::Any;

use crate::core::types::{TypeContext, Value};

pub trait NativeMethodWrapper: Send + Sync {
    fn invoke(&self, arguments: Vec<Value>, context: &mut dyn TypeContext) -> Result<Value, String>;
    fn argument_count(&self) -> i32;
}

pub trait ValueConverter: Send + Sync {
    fn convert_from_gvm(&self, context: &dyn TypeContext, value: &Value) -> Box<dyn Any>;
    fn convert_from_gvm_to(
        &self,
        context: &dyn TypeContext,
        value: &Value,
        target: &str,
    ) -> Box<dyn Any>;
    fn convert_to_gvm(&self, context: &mut dyn TypeContext, value: Box<dyn Any>) -> Value;
}
