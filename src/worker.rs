use deno_core::error::AnyError;
use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_runtime::deno_core;
use deno_runtime::deno_core::anyhow::anyhow;
use deno_runtime::deno_core::ModuleSpecifier;
use deno_runtime::deno_web::BlobStore;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use deno_runtime::BootstrapOptions;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;

use crate::loader::OnlyLoadWrapperImports;

const RUNTIME_VERSION: &'static str = "0.0.1";
const USER_AGENT: &'static str = "openedge-0.0.1";

fn get_error_class_name(e: &AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
}

pub fn instance(main_module: ModuleSpecifier, port: u16) -> Result<MainWorker, AnyError> {
    let module_loader = Rc::new(OnlyLoadWrapperImports::new());
    let create_web_worker_cb = Arc::new(|_| unimplemented!());
    let web_worker_event_cb = Arc::new(|_| unimplemented!());

    let options = WorkerOptions {
        bootstrap: BootstrapOptions {
            args: vec![],
            cpu_count: 1,
            debug_flag: false,
            enable_testing_features: false,
            location: None,
            no_color: false,
            is_tty: false,
            runtime_version: RUNTIME_VERSION.to_string(),
            ts_version: "-".to_string(),
            unstable: true,
            user_agent: USER_AGENT.to_string(),
            inspect: false,
        },
        extensions: vec![],
        unsafely_ignore_certificate_errors: None,
        root_cert_store: None,
        seed: None,
        source_map_getter: None,
        format_js_error_fn: None,
        web_worker_preload_module_cb: web_worker_event_cb.clone(),
        web_worker_pre_execute_module_cb: web_worker_event_cb,
        create_web_worker_cb,
        maybe_inspector_server: None,
        should_break_on_first_statement: false,
        module_loader,
        npm_resolver: None,
        get_error_class_fn: Some(&get_error_class_name),
        origin_storage_dir: None,
        blob_store: BlobStore::default(),
        broadcast_channel: InMemoryBroadcastChannel::default(),
        shared_array_buffer_store: None,
        compiled_wasm_module_store: None,
        stdio: Default::default(),
    };

    let permissions = crate::permissions::permissions(port);
    let worker = MainWorker::bootstrap_from_options(main_module, permissions, options);
    Ok(worker)
}

// TODO: this is very important (with respect to cold start times) and is currently extremely
// non-optimal.
pub async fn wait_until_dials(addr: SocketAddr) -> Result<(), AnyError> {
    // TODO: add loop timeout
    loop {
        match tokio::net::TcpStream::connect(addr).await {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == ErrorKind::ConnectionRefused => continue,
            Err(e) => return Err(anyhow!("worker failed readiness probe with bad error: {e}")),
        }
    }
}
