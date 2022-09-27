use crate::{startup_new_worker, worker::wait_until_dials, Worker, Workers};
use deno_runtime::deno_core::anyhow::anyhow;
use deno_runtime::deno_core::anyhow::{bail, Error};
use hyper::{Body, Request};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub async fn resolve_to_proxy(mut state: Workers, req: &Request<Body>) -> Result<String, Error> {
    let host = req
        .headers()
        .get("host")
        .ok_or(anyhow!("\"host\" header not found"))?
        .to_str()?
        .split('.')
        .next()
        .ok_or(anyhow!("invalid host header"))?;

    let worker = {
        match state.get_existing_worker_port(host) {
            Some(port) => Worker { port },
            None => {
                let main_module = match state.store.hostname_to_module(host.to_string()) {
                    Ok(m) => m,
                    Err(_e) => bail!("invalid host header"),
                };
                let new_worker = startup_new_worker(&mut state, main_module).await?;
                let before_coldstart = tokio::time::Instant::now();
                state.register_new_running_worker(host, new_worker.clone());
                wait_until_dials(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    new_worker.port,
                ))
                .await
                .expect("worker failed to get ready");
                println!(
                    "cold start took = {}ms",
                    before_coldstart.elapsed().as_millis()
                );
                new_worker
            }
        }
    };
    Ok(format!("http://127.0.0.1:{}", worker.port))
}