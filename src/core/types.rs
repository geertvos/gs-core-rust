use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
    And,
    Or,
    Not,
    Eql,
    Lt,
    Gt,
    Get,
    Invoke,
    New,
}

pub trait TypeContext {
    fn heap_get_object_value(&self, obj_ref: i32, field: &str) -> Option<Value>;
    fn heap_set_object_value(&mut self, obj_ref: i32, field: &str, value: Value);
    fn heap_has_object_value(&self, obj_ref: i32, field: &str) -> bool;
    fn heap_add_object(&mut self) -> i32;
    fn heap_add_object_box(&mut self, _object: Box<dyn crate::core::object::GvmObject>) -> i32 {
        panic!("heap_add_object_box not supported in this context")
    }
    fn heap_get_object_keys(&self, obj_ref: i32) -> Vec<String>;
    fn heap_get_object_any(&self, _obj_ref: i32) -> Option<&dyn std::any::Any> {
        None
    }
    fn get_string(&self, index: i32) -> Option<&str>;
    fn add_string(&mut self, _s: &str) -> i32 {
        panic!("add_string not supported in this context")
    }

    fn generate_native_method_function(
        &mut self,
        _wrapper: Box<dyn crate::bridge::NativeMethodWrapper>,
        _arg_count: i32,
    ) -> i32 {
        panic!("generate_native_method_function not supported in this context")
    }
}

pub trait Type: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn supports_operation(&self, op: Operation) -> bool;
    fn perform(
        &self,
        context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value;
    fn is_instance(&self, other: &dyn Type) -> bool;
    fn clone_type(&self) -> Box<dyn Type>;
}

impl Clone for Box<dyn Type> {
    fn clone(&self) -> Self {
        self.clone_type()
    }
}

#[derive(Debug, Clone)]
pub enum ValueSource {
    None,
    Stack(usize),
    HeapField(i32, String),
}

#[derive(Debug, Clone)]
pub struct Value {
    pub value: i32,
    pub type_: Box<dyn Type>,
    pub comment: Option<String>,
    pub source: ValueSource,
}

impl Value {
    pub fn new(value: i32, type_: Box<dyn Type>) -> Self {
        Value {
            value,
            type_,
            comment: None,
            source: ValueSource::None,
        }
    }

    pub fn with_comment(value: i32, type_: Box<dyn Type>, comment: String) -> Self {
        Value {
            value,
            type_,
            comment: Some(comment),
            source: ValueSource::None,
        }
    }

    pub fn undefined() -> Self {
        Value {
            value: 0,
            type_: Box::new(UndefinedType),
            comment: None,
            source: ValueSource::None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.comment {
            Some(c) => write!(f, "{}:{} //{}", self.value, self.type_.name(), c),
            None => write!(f, "{}:{}", self.value, self.type_.name()),
        }
    }
}

// --- Built-in types ---

#[derive(Debug, Clone)]
pub struct UndefinedType;

impl Type for UndefinedType {
    fn name(&self) -> &str {
        "Undefined"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(op, Operation::Eql)
    }

    fn perform(
        &self,
        _context: &mut dyn TypeContext,
        op: Operation,
        _this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::Eql => {
                let result = match other_value {
                    Some(other) => other.type_.name() == "Undefined",
                    None => false,
                };
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            _ => panic!("Operation {:?} not supported on Undefined.", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(UndefinedType)
    }
}

#[derive(Debug, Clone)]
pub struct BooleanType;

impl Type for BooleanType {
    fn name(&self) -> &str {
        "Boolean"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(
            op,
            Operation::Not | Operation::And | Operation::Or | Operation::Eql
        )
    }

    fn perform(
        &self,
        _context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::Not => {
                let result = if this_value.value > 0 { 0 } else { 1 };
                Value::new(result, Box::new(BooleanType))
            }
            Operation::And => {
                let other = other_value.expect("AND requires two operands");
                let result = if this_value.value > 0 && other.value > 0 {
                    1
                } else {
                    0
                };
                Value::new(result, Box::new(BooleanType))
            }
            Operation::Or => {
                let other = other_value.expect("OR requires two operands");
                let result = if this_value.value > 0 || other.value > 0 {
                    1
                } else {
                    0
                };
                Value::new(result, Box::new(BooleanType))
            }
            Operation::Eql => {
                let other = other_value.expect("EQL requires two operands");
                let result = (this_value.value > 0) == (other.value > 0);
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            _ => panic!("Operation {:?} is not supported by type Boolean", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(BooleanType)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionType;

impl Type for FunctionType {
    fn name(&self) -> &str {
        "Function"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(op, Operation::Invoke | Operation::Eql)
    }

    fn perform(
        &self,
        _context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::Eql => {
                let other = other_value.expect("EQL requires two operands");
                let result = this_value.value == other.value;
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            _ => Value::new(0, Box::new(UndefinedType)),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(FunctionType)
    }
}
