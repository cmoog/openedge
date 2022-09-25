use std::collections::HashSet;

use deno_runtime::permissions::{NetDescriptor, Permissions, UnaryPermission};

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
