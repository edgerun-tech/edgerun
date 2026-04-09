//! End-to-end capability tests exercising real hardware through the
//! remote-capability protocol stack.
//!
//! Each test is guarded by a `#[ignore]` attribute and requires the
//! `HARDWARE_E2E=1` environment variable so they only run on developer
//! machines or CI nodes that have the physical devices.
//!
//! # Running
//!
//! ```bash
//! # Run all hardware e2e tests (requires real devices)
//! HARDWARE_E2E=1 cargo test -p edgerun-e2e-capability -- --ignored
//!
//! # Run a single test
//! HARDWARE_E2E=1 cargo test -p edgerun-e2e-capability test_input_device_e2e -- --ignored
//! ```

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` when the `HARDWARE_E2E=1` env var is set.
fn hardware_e2e_enabled() -> bool {
    std::env::var("HARDWARE_E2E").as_deref() == Ok("1")
}

/// Panics with a skip message if `HARDWARE_E2E` is not set, so individual
/// tests can call this at the top and `#[ignore]` keeps them out of the
/// default test suite.
fn require_hardware() {
    if !hardware_e2e_enabled() {
        panic!("skipping: set HARDWARE_E2E=1 to run hardware e2e tests");
    }
}

/// Convenience wrapper: builds a `PolicyWrappedProvider` around any
/// `RemoteCapabilityProvider` and opens a session over a Unix-socket
/// transport pair (in-memory via paired `UnixStream`s).
mod session_harness {
    use edgerun_capabilities::{CapabilityAccessClass, CapabilityOperation};
    use edgerun_proto::edgerun::v0::capability_runtime::{
        capability_remote_envelope, CapabilityRemoteEnvelope, CapabilitySessionMode,
        CapabilitySessionOpen,
    };
    use edgerun_remote_capability::{
        FramedRemoteTransport, PolicyWrappedProvider, RemoteCapabilityProvider,
        RemoteCapabilityTransport, serve_one,
    };
    use std::os::unix::net::UnixStream;
    use std::thread;

    /// Opens a session with the given provider over an in-process Unix socket pair.
    /// Returns the client transport and the session-id used.
    pub fn open_session_with_provider<P>(
        provider: P,
        session_id: &[u8],
    ) -> Result<FramedRemoteTransport<UnixStream>, Box<dyn std::error::Error>>
    where
        P: RemoteCapabilityProvider + Send + 'static,
    {
        let (client_stream, server_stream) = UnixStream::pair()?;
        let mut server_transport = FramedRemoteTransport::new(server_stream);

        // Spawn server in background thread
        thread::spawn(move || {
            let mut wrapped = PolicyWrappedProvider::new(provider);
            // serve_one handles: recv SessionOpen → send SessionAccept → recv Invocation → send Result
            // We only pump one request/response cycle per call.
            let _ = serve_one(&mut wrapped, &mut server_transport);
        });

        // Small sleep so server is ready
        thread::sleep(std::time::Duration::from_millis(20));

        let mut client_transport = FramedRemoteTransport::new(client_stream);

        // Client sends SessionOpen
        client_transport.send(CapabilityRemoteEnvelope {
            message: Some(capability_remote_envelope::Message::SessionOpen(
                CapabilitySessionOpen {
                    version: 1,
                    session_id: session_id.to_vec(),
                    selector: None,
                    mode: CapabilitySessionMode::Unary as i32,
                    requested_operations: vec![CapabilityOperation::Observe as i32],
                    requested_access_class: CapabilityAccessClass::Derived as i32,
                    requested_constraints: Vec::new(),
                    correlation_id: Vec::new(),
                },
            )),
        })?;

        // Wait for SessionAccept
        let _accept = client_transport
            .recv()?
            .ok_or("server closed before session accept")?;

        Ok(client_transport)
    }
}

// ---------------------------------------------------------------------------
// Test modules
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
