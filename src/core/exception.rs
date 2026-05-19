use super::types::{TypeContext, Value};

pub trait GvmExceptionHandler: Send + Sync {
    fn convert_message(
        &self,
        message: &str,
        context: &mut dyn TypeContext,
        line: i32,
        location: i32,
    ) -> Value;
    fn convert_value(
        &self,
        value: &Value,
        context: &mut dyn TypeContext,
        line: i32,
        location: i32,
    ) -> Value;
}

/// A placeholder exception handler used temporarily when the real handler
/// is taken out of the program for borrow-splitting purposes.
pub struct NoOpExceptionHandler;

impl GvmExceptionHandler for NoOpExceptionHandler {
    fn convert_message(
        &self,
        _message: &str,
        _context: &mut dyn TypeContext,
        _line: i32,
        _location: i32,
    ) -> Value {
        panic!("NoOpExceptionHandler should never be called")
    }

    fn convert_value(
        &self,
        _value: &Value,
        _context: &mut dyn TypeContext,
        _line: i32,
        _location: i32,
    ) -> Value {
        panic!("NoOpExceptionHandler should never be called")
    }
}
