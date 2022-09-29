use deno_core::anyhow;
use deno_core::error::AnyError;
use deno_runtime::deno_core::futures::TryFutureExt;
use deno_runtime::deno_core::{self, ModuleSpecifier};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, Ipv4Addr};
use std::rc::Rc;
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use worker::run_usercode;

pub mod loader;
pub mod router;
pub mod runtime;
pub mod store;
pub mod worker;

async fn handle(
    state: Workers,
    addr: SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    match router::resolve_to_proxy(state, &req).await {
        Ok(proxy_url) => {
            match hyper_reverse_proxy::call(addr.ip(), proxy_url.as_str(), req).await {
                Ok(resp) => Ok(resp),
                Err(_e) => Ok(Response::builder().status(500).body(Body::empty()).unwrap()),
            }
        }
        Err(_e) => Ok(Response::builder()
            .status(500)
            .body("\"host\" header not found".into())
            .unwrap()),
    }
}

async fn startup_new_worker(
    state: &mut Workers,
    host_slug: String,
    main_module: ModuleSpecifier,
) -> Result<Worker, AnyError> {
    let port = state
        .take_available_port()
        .ok_or_else(|| anyhow::anyhow!("ran out of ports!"))?;

    let state = state.clone();
    tokio::task::spawn_local(async move { 
        match run_usercode(main_module, port).await {
            Ok(()) => {},
            Err(e) => {
                println!("user code failed: {e}");
                state.running.borrow_mut().remove(&host_slug);
            },
        }
    });

    Ok(Worker { port })
}

#[derive(Clone, Debug)]
pub struct Worker {
    port: u16,
}

// TODO: fix this entire abstraction
#[derive(Clone, Debug)]
pub struct Workers {
    running: Rc<RefCell<HashMap<String, Worker>>>,
    available_ports: Rc<RefCell<BTreeSet<u16>>>,
    store: store::Store,
}

impl Workers {
    fn register_new_running_worker(&self, hostname: &str, worker: Worker) {
        self.running
            .borrow_mut()
            .insert(hostname.to_string(), worker);
    }
    fn take_available_port(&mut self) -> Option<u16> {
        let next_port = match self.available_ports.borrow().iter().next().cloned() {
            Some(p) => p,
            None => return None,
        };
        self.available_ports.borrow_mut().take(&next_port);
        Some(next_port)
    }

    fn get_existing_worker_port(&self, hostname: &str) -> Option<u16> {
        match self.running.borrow().get(hostname) {
            Some(w) => Some(w.port),
            None => None,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    let local = tokio::task::LocalSet::new();
    local.block_on(
        &rt,
        // TODO: rm need for panic
        startup_ingress().map_err(|e| panic!("failed to startup ingress: {e}")),
    )
}

async fn startup_ingress() -> Result<(), AnyError> {
    let mut store = store::Store::default();
    store.register_module("hello".to_string(), deno_core::resolve_path("./hello.js")?);
    store.register_module(
        "goodbye".to_string(),
        deno_core::resolve_path("./goodbye.js")?,
    );
    store.register_module("nice".to_string(), deno_core::resolve_path("./goodbye.js")?);

    let state = Workers {
        running: Rc::new(RefCell::new(HashMap::new())),
        available_ports: Rc::new(RefCell::new(BTreeSet::from([8888, 9999, 8081]))),
        store,
    };

    let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
    let listener = TcpListener::bind(addr).await?;
    println!("listening on {addr}");
    loop {
        let (stream, _) = listener.accept().await?;

        let state = state.clone();
        let service = service_fn(move |req| handle(state.clone(), addr, req));

        tokio::task::spawn_local(async move {
            if let Err(err) = Http::new()
                .with_executor(LocalExec)
                .serve_connection(stream, service)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

#[derive(Clone)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static, // !Send
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}
