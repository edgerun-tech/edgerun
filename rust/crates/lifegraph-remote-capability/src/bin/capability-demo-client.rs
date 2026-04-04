use lifegraph_capabilities::{CapabilityAccessClass, CapabilityOperation};
use lifegraph_remote_capability::{
    capability_remote_envelope, FramedRemoteTransport, RemoteCapabilityTransport,
};
use lifegraph_proto::lifegraph::v0::capability::CapabilityInvocation;
use lifegraph_proto::lifegraph::v0::capability_runtime::{
    CapabilityRemoteEnvelope, CapabilitySessionMode, CapabilitySessionOpen,
};
use std::env;

fn run_client<T: RemoteCapabilityTransport>(
    transport: &mut T,
) -> Result<(), Box<dyn std::error::Error>> {
    transport.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::SessionOpen(
            CapabilitySessionOpen {
                version: 1,
                session_id: b"demo-session".to_vec(),
                selector: None,
                mode: CapabilitySessionMode::Unary as i32,
                requested_operations: vec![CapabilityOperation::Observe as i32],
                requested_access_class: CapabilityAccessClass::Derived as i32,
                requested_constraints: Vec::new(),
                correlation_id: Vec::new(),
            },
        )),
    })?;

    let accept = transport.recv()?.ok_or("server closed before session accept")?;
    eprintln!("accept={accept:?}");

    transport.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::Invocation(
            CapabilityInvocation {
                invocation_version: 1,
                invocation_id: b"demo-invocation".to_vec(),
                grant_id: b"demo-session".to_vec(),
                invoker: None,
                operation: CapabilityOperation::Observe as i32,
                requested_access_class: CapabilityAccessClass::Derived as i32,
                parameter_object: None,
                correlation_id: Vec::new(),
                invoked_at: None,
                signature: None,
            },
        )),
    })?;

    let result = transport.recv()?.ok_or("server closed before result")?;
    println!("{result:?}");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or("usage: capability-demo-client <unix|tcp> <path|addr>")?;
    let target = args.next().ok_or("usage: capability-demo-client <unix|tcp> <path|addr>")?;

    match mode.as_str() {
        "unix" => {
            let mut transport = FramedRemoteTransport::connect_unix(&target)?;
            run_client(&mut transport)?;
        }
        "tcp" => {
            let mut transport = FramedRemoteTransport::connect_tcp(&target)?;
            run_client(&mut transport)?;
        }
        _ => return Err("mode must be 'unix' or 'tcp'".into()),
    }
    Ok(())
}
