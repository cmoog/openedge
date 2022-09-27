use std::collections::HashMap;

use deno_runtime::deno_core::anyhow;
use deno_runtime::deno_core::{error::AnyError, ModuleSpecifier};

#[derive(Default, Clone, Debug)]
pub struct Store {
    store: HashMap<String, ModuleSpecifier>,
}

impl Store {
    pub fn register_module(&mut self, host_slug: String, module: ModuleSpecifier) {
        self.store.insert(host_slug, module);
    }

    pub fn hostslug_to_module(&self, hostname: String) -> Result<ModuleSpecifier, AnyError> {
        self.store
            .get(&hostname)
            .ok_or_else(|| anyhow::anyhow!("hostname not found"))
            .map(|a| a.clone())
    }
}
