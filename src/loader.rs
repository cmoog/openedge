use deno_core::anyhow;
use deno_core::FsModuleLoader;
use deno_runtime::deno_core;
use deno_runtime::deno_core::error::generic_error;
use deno_runtime::deno_core::futures::FutureExt;
use deno_runtime::deno_core::ModuleLoader;
use deno_runtime::deno_core::ModuleSourceFuture;
use deno_runtime::deno_core::ModuleSpecifier;
use std::pin::Pin;

pub struct OnlyMainModuleLoader(FsModuleLoader);

impl OnlyMainModuleLoader {
    pub fn new() -> Self {
        OnlyMainModuleLoader(FsModuleLoader)
    }
}

impl ModuleLoader for OnlyMainModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        is_main: bool,
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        if is_main {
            self.0.resolve(specifier, referrer, is_main)
        } else {
            Err(generic_error("Module loading is not supported"))
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<ModuleSpecifier>,
        is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        if is_dyn_import {
            async { Err(generic_error("Dynamic import() statements not supported")) }.boxed_local()
        } else if module_specifier.scheme() != "file" {
            async { Err(generic_error("Main module must be a file path")) }.boxed_local()
        } else {
            self.0.load(module_specifier, maybe_referrer, is_dyn_import)
        }
    }
}
