// ADAPTED FROM: https://github.com/denoland/deno/blob/9c861ec4301397456e249923c881d9d3b56651f4/runtime
use std::rc::Rc;

use anyhow::anyhow;
use deno_flash::FlashPermissions;
use deno_runtime::deno_core::ModuleId;
use deno_runtime::deno_core::{
    self, anyhow, error::AnyError, Extension, JsRuntime, ModuleSpecifier, RuntimeOptions,
};
use deno_runtime::deno_fetch::FetchPermissions;
use deno_runtime::deno_net::NetPermissions;
use deno_runtime::deno_web::TimersPermission;
use deno_runtime::deno_websocket::WebSocketPermissions;
use deno_runtime::worker::WorkerOptions;
use deno_runtime::{
    deno_broadcast_channel, deno_console, deno_crypto, deno_fetch, deno_http, deno_net, deno_tls,
    deno_url, deno_web, deno_webgpu, deno_webidl, deno_websocket, deno_webstorage, ops,
};

use crate::located_script_name;

#[derive(Clone)]
pub struct Permissions {
    pub allow_local_port: u16,
}

impl TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        false
    }
    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {}
}

fn block_local_net<T: AsRef<str>>(hostname: &T) -> Result<(), AnyError> {
    match hostname.as_ref() {
        "localhost" | "127.0.0.1" | "[::1]" | "0.0.0.0" | "[::]" => {
            Err(anyhow!("local net blocked"))
        }
        _ => Ok(()),
    }
}

impl FlashPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), AnyError> {
        match block_local_net(&host.0) {
            Ok(()) => Ok(()),
            Err(_) if Some(self.allow_local_port) == host.1 => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl FetchPermissions for Permissions {
    fn check_net_url(
        &mut self,
        url: &deno_fetch::reqwest::Url,
        _api_name: &str,
    ) -> Result<(), AnyError> {
        block_local_net(&url.host().ok_or(anyhow!("no host"))?.to_string())
    }
    fn check_read(&mut self, _p: &std::path::Path, _api_name: &str) -> Result<(), AnyError> {
        Err(anyhow!("local reads not permitted"))
    }
}

impl WebSocketPermissions for Permissions {
    fn check_net_url(
        &mut self,
        url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), AnyError> {
        block_local_net(url)
    }
}

impl NetPermissions for Permissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), AnyError> {
        match block_local_net(&host.0) {
            Ok(()) => Ok(()),
            Err(_) if Some(self.allow_local_port) == host.1 => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn check_read(&mut self, _p: &std::path::Path, _api_name: &str) -> Result<(), AnyError> {
        Err(anyhow!("local reads not permitted"))
    }

    fn check_write(&mut self, _p: &std::path::Path, _api_name: &str) -> Result<(), AnyError> {
        Err(anyhow!("local writes not permitted"))
    }
}

pub struct Runtime {
    pub js_runtime: JsRuntime,
}

impl Runtime {
    pub fn bootstrap_from_options(
        main_module: ModuleSpecifier,
        permissions: Permissions,
        options: WorkerOptions,
    ) -> Self {
        let bootstrap_options = options.bootstrap.clone();
        let mut worker = Self::from_options(main_module, permissions, options);
        worker.bootstrap(&bootstrap_options);
        worker
    }
    pub fn from_options(
        main_module: ModuleSpecifier,
        permissions: Permissions,
        mut options: WorkerOptions,
    ) -> Self {
        // Permissions: many ops depend on this
        let unstable = options.bootstrap.unstable;
        // let enable_testing_features = options.bootstrap.enable_testing_features;
        let perm_ext = Extension::builder()
            .state(move |state| {
                state.put::<Permissions>(permissions.clone());
                state.put(ops::UnstableChecker { unstable });
                // state.put(ops::TestingFeaturesEnabled(enable_testing_features));
                Ok(())
            })
            .build();
        // let exit_code = ExitCode::default();

        // Internal modules
        let mut extensions: Vec<Extension> = vec![
            // Web APIs
            deno_webidl::init(),
            deno_console::init(),
            deno_url::init(),
            deno_web::init::<Permissions>(
                options.blob_store.clone(),
                options.bootstrap.location.clone(),
            ),
            deno_fetch::init::<Permissions>(deno_fetch::Options {
                user_agent: options.bootstrap.user_agent.clone(),
                root_cert_store: options.root_cert_store.clone(),
                unsafely_ignore_certificate_errors: options
                    .unsafely_ignore_certificate_errors
                    .clone(),
                file_fetch_handler: Rc::new(deno_fetch::FsFetchHandler),
                ..Default::default()
            }),
            // deno_cache::init::<SqliteBackedCache>(create_cache),
            deno_websocket::init::<Permissions>(
                options.bootstrap.user_agent.clone(),
                options.root_cert_store.clone(),
                options.unsafely_ignore_certificate_errors.clone(),
            ),
            deno_webstorage::init(options.origin_storage_dir.clone()),
            deno_broadcast_channel::init(options.broadcast_channel.clone(), unstable),
            deno_crypto::init(options.seed),
            deno_webgpu::init(unstable),
            // ffi
            // deno_ffi::init::<Permissions>(unstable),
            // Runtime ops
            ops::runtime::init(main_module.clone()),
            // ops::worker_host::init(
            //     options.create_web_worker_cb.clone(),
            //     options.web_worker_preload_module_cb.clone(),
            //     options.web_worker_pre_execute_module_cb.clone(),
            //     options.format_js_error_fn.clone(),
            // ),
            // ops::spawn::init(),
            // ops::fs_events::init(),
            // ops::fs::init(),
            // ops::io::init(),
            ops::io::init_stdio(options.stdio),
            deno_tls::init(),
            deno_net::init::<Permissions>(
                options.root_cert_store.clone(),
                unstable,
                options.unsafely_ignore_certificate_errors.clone(),
            ),
            // deno_node::init::<Permissions>(unstable, options.npm_resolver),
            // ops::os::init(exit_code.clone()),
            // ops::permissions::init(),
            // ops::process::init(),
            // ops::signal::init(),
            // ops::tty::init(),
            deno_http::init(),
            deno_flash::init::<Permissions>(unstable),
            ops::http::init(),
            // Permissions ext (worker specific state)
            perm_ext,
        ];
        extensions.extend(std::mem::take(&mut options.extensions));

        let js_runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(options.module_loader.clone()),
            startup_snapshot: Some(deno_runtime::js::deno_isolate_init()),
            // startup_snapshot: None,
            source_map_getter: options.source_map_getter,
            get_error_class_fn: options.get_error_class_fn,
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: options.compiled_wasm_module_store.clone(),
            extensions,
            ..Default::default()
        });

        // if let Some(server) = options.maybe_inspector_server.clone() {
        //     server.register_inspector(
        //         main_module.to_string(),
        //         &mut js_runtime,
        //         options.should_break_on_first_statement,
        //     );
        // }

        Self { js_runtime }
    }

    pub async fn run_event_loop(&mut self, wait_for_inspector: bool) -> Result<(), AnyError> {
        self.js_runtime.run_event_loop(wait_for_inspector).await
    }

    pub fn bootstrap(&mut self, options: &deno_runtime::BootstrapOptions) {
        let script = format!("bootstrap.mainRuntime({})", options.as_json());
        self.execute_script(&located_script_name!(), &script)
            .expect("Failed to execute bootstrap script");
    }

    /// See [JsRuntime::execute_script](deno_core::JsRuntime::execute_script)

    pub fn execute_script(&mut self, script_name: &str, source_code: &str) -> Result<(), AnyError> {
        self.js_runtime.execute_script(script_name, source_code)?;
        Ok(())
    }

    /// Executes specified JavaScript module.
    pub async fn evaluate_module(&mut self, id: ModuleId) -> Result<(), AnyError> {
        let mut receiver = self.js_runtime.mod_evaluate(id);
        tokio::select! {
          // Not using biased mode leads to non-determinism for relatively simple
          // programs.
          biased;

          maybe_result = &mut receiver => {
            maybe_result.expect("Module evaluation result not provided.")
          }

          event_loop_result = self.run_event_loop(false) => {
            event_loop_result?;
            let maybe_result = receiver.await;
            maybe_result.expect("Module evaluation result not provided.")
          }
        }
    }
}

/// A helper macro that will return a call site in Rust code. Should be
/// used when executing internal one-line scripts for JsRuntime lifecycle.
///
/// Returns a string in form of: "`[deno:<filename>:<line>:<column>]`"
#[macro_export]
macro_rules! located_script_name {
    () => {
        format!(
            "[deno:{}:{}:{}]",
            std::file!(),
            std::line!(),
            std::column!()
        )
    };
}
