use deno_core::error::AnyError;
use deno_runtime::deno_core;

pub mod loader;
pub mod permissions;
pub mod worker;
pub mod router;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let main_module = deno_core::resolve_path("./hello.js")?;
    let mut worker = worker::instance(main_module.clone())?;
    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;

    Ok(())
}
