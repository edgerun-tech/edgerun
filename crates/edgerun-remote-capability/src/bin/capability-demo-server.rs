use edgerun_capabilities::{
    capability_descriptor, CapabilityAccessClass, CapabilityDescriptor, CapabilityError,
    CapabilityEventKind, CapabilityModality, CapabilityOperation, CapabilityRole,
};
use edgerun_remote_capability::{
    accept_session_open_unchecked, accept_tcp, accept_unix, serve_one, PolicyWrappedProvider,
    RemoteCapabilityProvider, RemoteInvocationResult,
};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionClose, CapabilitySessionEvent, CapabilitySessionOpen,
};
use std::env;
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::Path;

struct DemoProvider;

impl RemoteCapabilityProvider for DemoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        capability_descriptor(
            "demo-remote-capability",
            "demo-provider",
            CapabilityRole::Communication,
            &[CapabilityModality::Text],
            &[CapabilityEventKind::Text],
            &[CapabilityOperation::Observe],
            Vec::new(),
        )
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<edgerun_remote_capability::CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: CapabilityAccessClass::Derived as i32,
                produced_event_kinds: vec![CapabilityEventKind::Text as i32],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload: b"demo-ok".to_vec(),
        })
    }

    fn next_event(
        &mut self,
        _session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        Ok(None)
    }

    fn close_session(&mut self, _close: &CapabilitySessionClose) -> Result<(), CapabilityError> {
        Ok(())
    }
}

fn run_one<T: edgerun_remote_capability::RemoteCapabilityTransport>(
    transport: &mut T,
) -> Result<(), CapabilityError> {
    let mut provider = PolicyWrappedProvider::new(DemoProvider);
    while serve_one(&mut provider, transport)? {}
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or("usage: capability-demo-server <unix|tcp> <path|addr>")?;
    let target = args.next().ok_or("usage: capability-demo-server <unix|tcp> <path|addr>")?;

    match mode.as_str() {
        "unix" => {
            let path = Path::new(&target);
            let _ = std::fs::remove_file(path);
            let listener = UnixListener::bind(path)?;
            eprintln!("listening on unix:{}", path.display());
            let mut transport = accept_unix(&listener)?;
            run_one(&mut transport)?;
        }
        "tcp" => {
            let listener = TcpListener::bind(&target)?;
            eprintln!("listening on tcp:{}", target);
            let mut transport = accept_tcp(&listener)?;
            run_one(&mut transport)?;
        }
        _ => return Err("mode must be 'unix' or 'tcp'".into()),
    }
    Ok(())
}
