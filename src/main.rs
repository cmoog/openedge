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
use worker::wait_until_dials;

pub mod loader;
pub mod permissions;
pub mod store;
pub mod worker;

// TODO: wrap with a normal Result so the control flow can be cleaner with ? try syntax,
// then the handler just writes 500 when an error is returned.
async fn handle(
    mut state: Workers,
    addr: SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    match req.headers().get("host") {
        Some(host) => {
            let host = match host.to_str() {
                Ok(h) => h,
                Err(_e) => {
                    return Ok(Response::builder()
                        .status(500)
                        .body("invalid host header".into())
                        .unwrap())
                }
            };
            let worker = {
                match state.get_existing_worker_port(host) {
                    Some(port) => Worker { port },
                    None => {
                        let main_module = match state.store.hostname_to_module(host.to_string()) {
                            Ok(m) => m,
                            Err(_e) => {
                                return Ok(Response::builder()
                                    .status(500)
                                    .body("invalid host header".into())
                                    .unwrap())
                            }
                        };
                        match startup_new_worker(&mut state, main_module).await {
                            Ok(w) => {
                                let before_coldstart = tokio::time::Instant::now();
                                state.register_new_running_worker(host, w.clone());
                                wait_until_dials(SocketAddr::new(
                                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                                    w.port,
                                ))
                                .await
                                .expect("worker failed to get ready");
                                println!(
                                    "cold start took = {}ms",
                                    before_coldstart.elapsed().as_millis()
                                );
                                w
                            }
                            Err(e) => {
                                dbg!(e);
                                return Ok(Response::builder()
                                    .status(500)
                                    .body("invalid host header".into())
                                    .unwrap());
                            }
                        }
                    }
                }
            };

            match hyper_reverse_proxy::call(
                addr.ip(),
                format!("http://127.0.0.1:{}", worker.port).as_str(),
                req,
            )
            .await
            {
                Ok(resp) => Ok(resp),
                Err(_e) => Ok(Response::builder().status(500).body(Body::empty()).unwrap()),
            }
        }
        None => Ok(Response::builder()
            .status(500)
            .body("\"host\" header not found".into())
            .unwrap()),
    }
}

async fn startup_new_worker(
    state: &mut Workers,
    main_module: ModuleSpecifier,
) -> Result<Worker, AnyError> {
    let port = state
        .take_available_port()
        .ok_or_else(|| anyhow::anyhow!("ran out of ports!"))?;

    tokio::task::spawn_local(async move {
        let mut worker =
            worker::instance(main_module.clone(), port).expect("create new worker instance");
        worker
            .execute_main_module(&main_module)
            .await
            .expect("execute main module");
        worker.run_event_loop(false).await.expect("run event loop");
    });

    Ok(Worker { port })
}

#[derive(Clone, Debug)]
struct Worker {
    port: u16,
}

// TODO: fix this entire abstraction
#[derive(Clone, Debug)]
struct Workers {
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
