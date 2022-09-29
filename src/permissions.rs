use deno_runtime::{permissions::{NetDescriptor, Permissions, UnaryPermission}, deno_core::{Extension, error::AnyError}, deno_fetch::{self, FetchPermissions}};
use std::collections::HashSet;
use deno_runtime::deno_core::anyhow::anyhow;

pub struct FetchBlockLocal;

impl FetchPermissions for FetchBlockLocal {
    fn check_net_url(&mut self, url: &deno_fetch::reqwest::Url) -> Result<(), AnyError> {
        let host = url.host().ok_or(anyhow!("no host"))?.to_string();
        match host.as_str() {
            "localhost" | "127.0.0.1" | "[::1]" | "0.0.0.0" | "[::]" => {
                Err(anyhow!("local net blocked"))
            }
            _ => Ok(()),
        }
    }
    fn check_read(&mut self, _p: &std::path::Path) -> Result<(), AnyError> {
        Err(anyhow!("local reads not permitted"))
    }
}

pub fn perm_ext() -> Extension {
    Extension::builder()
        .state(move |state| {
            state.put::<FetchBlockLocal>(FetchBlockLocal);
            Ok(())
        })
        .build()
}

// specify a single local port to which the module will have access
pub fn permissions(local_port: u16) -> Permissions {
    let read = Permissions::new_read(&None, false).unwrap();
    let write = Permissions::new_write(&None, false).unwrap();

    // TODO: allow access to external network, block local dns
    let net = UnaryPermission::<NetDescriptor> {
        granted_list: HashSet::from([
            NetDescriptor("localhost".to_string(), Some(local_port)),
            NetDescriptor("0.0.0.0".to_string(), Some(local_port)),
            NetDescriptor("127.0.0.1".to_string(), Some(local_port)),
            NetDescriptor("[::]".to_string(), Some(local_port)),
            NetDescriptor("[::1]".to_string(), Some(local_port)),
        ]),
        global_state: deno_runtime::permissions::PermissionState::Denied,
        prompt: false,
        ..Default::default()
    };

    // We don't want to allow op access to *real* environment variables.
    // Instead, we bootstrap the environment with a mocked interface.
    let env = Permissions::new_env(&None, false).unwrap();
    let run = Permissions::new_run(&None, false).unwrap();
    let ffi = Permissions::new_ffi(&None, false).unwrap();
    let hrtime = Permissions::new_hrtime(false);

    Permissions {
        read,
        write,
        net,
        env,
        run,
        ffi,
        hrtime,
    }
}
