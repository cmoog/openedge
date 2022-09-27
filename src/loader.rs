use deno_core::anyhow;
use deno_core::FsModuleLoader;
use deno_runtime::deno_core;
use deno_runtime::deno_core::error::generic_error;
use deno_runtime::deno_core::futures::FutureExt;
use deno_runtime::deno_core::ModuleLoader;
use deno_runtime::deno_core::ModuleSourceFuture;
use deno_runtime::deno_core::ModuleSpecifier;
use std::pin::Pin;

pub struct UserModuleWrapper {
    pub code: String,
    pub spec: ModuleSpecifier,
}

const WRAPPER_MODULE_SPEC: &str = "file:///wrapper.js";

pub fn new_wrapper(user_module: &ModuleSpecifier) -> UserModuleWrapper {
    let code = format!(
        "import worker from \"{}\"; 
Deno.serve(worker.fetch, {{
    hostname: \"0.0.0.0\",
    port: Deno.env.get(\"PORT\"),
}})
",
        user_module.as_str()
    );
    let spec = deno_core::resolve_url(WRAPPER_MODULE_SPEC).unwrap();

    UserModuleWrapper { code, spec }
}

pub struct OnlyLoadWrapperImports(FsModuleLoader);

impl OnlyLoadWrapperImports {
    pub fn new() -> Self {
        OnlyLoadWrapperImports(FsModuleLoader)
    }
}

impl ModuleLoader for OnlyLoadWrapperImports {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        is_main: bool,
    ) -> Result<ModuleSpecifier, anyhow::Error> {
        if (is_main && specifier == WRAPPER_MODULE_SPEC) || referrer == WRAPPER_MODULE_SPEC {
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
