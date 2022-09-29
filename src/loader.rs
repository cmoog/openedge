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

pub fn new_wrapper<'a, E: IntoIterator<Item = &'a (&'a str, &'a str)>>(
    user_module: &ModuleSpecifier,
    env_vars: E,
    port: u16,
) -> UserModuleWrapper {
    let code = format!(
        "import worker from \"{}\"; 
Deno.serve((req) => worker.fetch(req, {{
    {}
}}), {{
    hostname: \"0.0.0.0\",
    port: \"{}\",
}})
",
        user_module.as_str(),
        to_js_keyvalues(env_vars),
        port
    );
    let spec = deno_core::resolve_url(WRAPPER_MODULE_SPEC).unwrap();

    UserModuleWrapper { code, spec }
}

fn to_js_keyvalues<'a, T: IntoIterator<Item = &'a (&'a str, &'a str)>>(key_pairs: T) -> String {
    key_pairs
        .into_iter()
        .map(|(key, value)| format!("\"{key}\": \"{value}\""))
        .collect::<Vec<String>>()
        .join(",\n")
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
