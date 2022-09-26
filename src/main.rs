use deno_core::anyhow;
use deno_core::error::AnyError;
use deno_runtime::deno_core::futures::TryFutureExt;
use deno_runtime::deno_core::{self, ModuleSpecifier};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response};
use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;

pub mod loader;
pub mod permissions;
pub mod router;
pub mod worker;

async fn handle(
    mut state: Workers,
    addr: SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    match req.headers().get("host") {
        Some(host) => {
            let host = match host.to_str() {
                Ok(h) => { h }
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
                        let main_module = deno_core::resolve_path(format!("{}.js", host).as_str())
                            .expect("handle");
                        match startup_new_worker(&mut state, main_module).await {
                            Ok(w) => {
                                {
                                    let mut runners = state.running.lock().unwrap();
                                    runners.insert(host.to_string(), w.clone());
                                    drop(runners);
                                };
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
    // module: ModuleSpecifier,
}

#[derive(Clone, Debug)]
struct Workers {
    running: Arc<Mutex<HashMap<String, Worker>>>,
    available_ports: Arc<Mutex<BTreeSet<u16>>>,
}

impl Workers {
    fn take_available_port(&mut self) -> Option<u16> {
        let mut ports = self.available_ports.lock().unwrap();
        let next_port = match ports.iter().next().cloned() {
            Some(p) => p,
            None => return None,
        };
        ports.take(&next_port);
        Some(next_port)
    }

    fn get_existing_worker_port(&self, hostname: &str) -> Option<u16> {
        let running_workers = self.running.lock().unwrap();
        match running_workers.get(hostname) {
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
        startup_ingress().map_err(|_| panic!("failed to startup ingress")),
    )
}

async fn startup_ingress() -> Result<(), AnyError> {
    let state = Workers {
        running: Arc::new(Mutex::new(HashMap::new())),
        available_ports: Arc::new(Mutex::new(BTreeSet::new())),
    };
    {
        let mut ports = state.available_ports.lock().unwrap();
        ports.insert(9090);
        ports.insert(8888);
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

// configure an Executor that can spawn !Send futures
#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static,
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}
