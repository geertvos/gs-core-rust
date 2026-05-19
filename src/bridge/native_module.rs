use std::fmt;

#[derive(Debug, Clone)]
pub struct NativeError {
    pub message: String,
}

impl NativeError {
    pub fn new(message: impl Into<String>) -> Self {
        NativeError {
            message: message.into(),
        }
    }
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type NativeResult = Result<NativeValue, NativeError>;

#[derive(Debug, Clone)]
pub struct MethodDescriptor {
    pub name: String,
    pub arg_count: i32,
}

impl MethodDescriptor {
    pub fn new(name: impl Into<String>, arg_count: i32) -> Self {
        MethodDescriptor {
            name: name.into(),
            arg_count,
        }
    }
}

pub enum NativeValue {
    Undefined,
    Number(i32),
    Boolean(bool),
    String(String),
    Instance(Box<dyn NativeInstance>),
    Bytes(Vec<u8>),
}

impl fmt::Debug for NativeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NativeValue::Undefined => write!(f, "Undefined"),
            NativeValue::Number(n) => write!(f, "Number({})", n),
            NativeValue::Boolean(b) => write!(f, "Boolean({})", b),
            NativeValue::String(s) => write!(f, "String({:?})", s),
            NativeValue::Instance(i) => write!(f, "Instance({})", i.type_name()),
            NativeValue::Bytes(b) => write!(f, "Bytes(len={})", b.len()),
        }
    }
}

pub trait NativeModule: Send + Sync {
    fn class_name(&self) -> &str;
    fn constructor(&self, args: Vec<NativeValue>) -> NativeResult;
    fn call_static(&self, method: &str, args: Vec<NativeValue>) -> NativeResult;
    fn static_methods(&self) -> Vec<MethodDescriptor>;
}

pub trait NativeInstance: Send + Sync {
    fn type_name(&self) -> &str;
    fn instance_methods(&self) -> Vec<MethodDescriptor>;
    fn call_method(&self, method: &str, args: Vec<NativeValue>) -> NativeResult;
    fn destroy(&self);
    fn clone_instance(&self) -> Box<dyn NativeInstance>;
    fn as_any(&self) -> &dyn std::any::Any;
}
