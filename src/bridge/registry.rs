use std::collections::HashMap;
use std::sync::Arc;

use super::native_module::{NativeModule, NativeResult, NativeValue, NativeError};

pub struct NativeRegistry {
    modules: HashMap<String, Arc<dyn NativeModule>>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        NativeRegistry {
            modules: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: Arc<dyn NativeModule>) {
        let name = module.class_name().to_string();
        self.modules.insert(name, module);
    }

    pub fn dispatch(&self, class: &str, method: &str, args: Vec<NativeValue>) -> NativeResult {
        let module = self
            .modules
            .get(class)
            .ok_or_else(|| NativeError::new(format!("Native module not found: {}", class)))?;

        let simple_name = class.rsplit('.').next().unwrap_or(class);
        if method == simple_name {
            module.constructor(args)
        } else {
            module.call_static(method, args)
        }
    }

    pub fn registered_classes(&self) -> Vec<&str> {
        self.modules.keys().map(|k| k.as_str()).collect()
    }
}
