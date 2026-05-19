use std::any::Any;

use super::types::Value;

pub trait GvmObject: Send + Sync {
    fn set_value(&mut self, id: &str, v: Value);
    fn get_value(&self, id: &str) -> Option<Value>;
    fn has_value(&self, id: &str) -> bool;
    fn get_values(&self) -> Vec<Value>;
    fn get_keys(&self) -> Vec<String>;
    fn pre_destroy(&self);
    fn clone_object(&self) -> Box<dyn GvmObject>;
    fn as_any(&self) -> &dyn Any {
        panic!("as_any not supported")
    }
}
