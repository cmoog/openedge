use crate::{startup_new_worker, worker::wait_until_dials, IsolateManager, RunningIsolateMetadata};
use deno_runtime::deno_core::anyhow::anyhow;
use deno_runtime::deno_core::anyhow::Error;
use hyper::{Body, Request};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub async fn resolve_to_proxy(
    mut state: IsolateManager,
    req: &Request<Body>,
) -> Result<String, Error> {
    let headers = req.headers();
    let header_value = match headers.get(":authority") {
        Some(a) => a,
        None => headers.get("host").ok_or(anyhow!(
            "neither \"host\" nor \":authority\" found in header"
        ))?,
    };
    let host_slug = header_value
        .to_str()?
        .split('.')
        .next()
        .ok_or(anyhow!("invalid host header"))?;

    let worker = {
        match state.get_running_isolate_port(host_slug) {
            Some(port) => RunningIsolateMetadata { port },
            None => {
                let main_module = state.store.hostslug_to_module(host_slug.to_string())?;
                let new_worker =
                    startup_new_worker(&mut state, host_slug.to_string(), main_module).await?;
                let before_coldstart = tokio::time::Instant::now();
                wait_until_dials(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    new_worker.port,
                ))
                .await?;
                println!(
                    "cold start took = {}ms",
                    before_coldstart.elapsed().as_millis()
                );
                state.register_new_isolate(host_slug, new_worker.clone());
                new_worker
            }
        }
    };
    Ok(format!("http://127.0.0.1:{}", worker.port))
}
