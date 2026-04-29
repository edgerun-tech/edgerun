//! End-to-end capability tests exercising real hardware through the
//! remote-capability protocol stack.
//!
//! Hardware-backed tests return early unless `HARDWARE_E2E=1` is set, so the
//! suite reports zero ignored tests while still avoiding accidental device use.
//!
//! # Running
//!
//! ```bash
//! HARDWARE_E2E=1 cargo test -p edgerun-e2e-capability
//! ```

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::boxed::Box;
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns whether hardware-backed e2e tests should touch real devices.
#[cfg(not(target_os = "none"))]
fn require_hardware() -> bool {
    if std::env::var("HARDWARE_E2E").as_deref() != Ok("1") {
        std::println!("skipping hardware e2e test: set HARDWARE_E2E=1 to run against devices");
        return false;
    }
    true
}

/// Test policy: auto-grants every session open with all requested operations.
/// This wraps any `RemoteCapabilityProvider` and bypasses the grant-request
/// cycle so sessions always succeed.
pub(crate) mod test_policy {
    use crate::prelude::v1::*;
    use edgerun_capabilities::{CapabilityError, CapabilityInvocation};
    use edgerun_proto::edgerun::v0::capability_runtime::{
        CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionEvent,
        CapabilitySessionOpen,
    };
    use edgerun_remote_capability::{
        accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult,
    };

    /// Wraps a provider and auto-accepts every session with all requested
    /// operations granted.
    pub struct TestGrantedProvider<P> {
        inner: P,
    }

    impl<P> TestGrantedProvider<P> {
        pub fn new(inner: P) -> Self {
            Self { inner }
        }
    }

    impl<P: RemoteCapabilityProvider> RemoteCapabilityProvider for TestGrantedProvider<P> {
        fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
            self.inner.descriptor()
        }

        fn open_session(
            &mut self,
            open: &CapabilitySessionOpen,
        ) -> Result<CapabilitySessionAccept, CapabilityError> {
            // Accept unconditionally with all requested operations
            let mut accept = accept_session_open_unchecked(open);
            accept.granted_operations = open.requested_operations.clone();
            accept.granted_access_class = open.requested_access_class;
            Ok(accept)
        }

        fn invoke(
            &mut self,
            session_id: &[u8],
            invocation: &CapabilityInvocation,
            inline_parameters: Option<&[u8]>,
        ) -> Result<RemoteInvocationResult, CapabilityError> {
            self.inner.invoke(session_id, invocation, inline_parameters)
        }

        fn next_event(
            &mut self,
            session_id: &[u8],
        ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
            self.inner.next_event(session_id)
        }

        fn close_session(&mut self, close: &CapabilitySessionClose) -> Result<(), CapabilityError> {
            self.inner.close_session(close)
        }
    }
}

/// Session harness: opens a session with a provider over an in-process
/// Unix socket pair (server thread + client transport).
#[cfg(not(target_os = "none"))]
pub(crate) mod session_harness {
    use crate::prelude::v1::*;
    use edgerun_capabilities::{CapabilityAccessClass, CapabilityOperation};
    use edgerun_proto::edgerun::v0::capability_runtime::{
        capability_remote_envelope, CapabilityRemoteEnvelope, CapabilitySessionMode,
        CapabilitySessionOpen,
    };
    use edgerun_remote_capability::{
        serve_one, FramedRemoteTransport, RemoteCapabilityProvider, RemoteCapabilityTransport,
    };
    use std::os::unix::net::UnixStream;
    use std::thread;

    /// Opens a session with the given provider over a Unix socket pair.
    /// The server runs `serve_one()` in a background thread; the client
    /// sends `SessionOpen` and waits for `SessionAccept`.
    pub fn open_session<P>(
        provider: P,
        session_id: &[u8],
    ) -> Result<FramedRemoteTransport<UnixStream>, Box<dyn std::error::Error>>
    where
        P: RemoteCapabilityProvider + Send + 'static,
    {
        let (client_sock, server_sock) = UnixStream::pair()?;
        let mut server_transport = FramedRemoteTransport::new(server_sock);

        thread::spawn(move || {
            let _ = serve_one(
                &mut super::test_policy::TestGrantedProvider::new(provider),
                &mut server_transport,
            );
        });

        // Let the server thread start
        thread::sleep(std::time::Duration::from_millis(20));

        let mut client_transport = FramedRemoteTransport::new(client_sock);

        client_transport.send(CapabilityRemoteEnvelope {
            message: Some(capability_remote_envelope::Message::SessionOpen(
                CapabilitySessionOpen {
                    version: 1,
                    session_id: session_id.to_vec(),
                    selector: None,
                    mode: CapabilitySessionMode::Stream as i32,
                    requested_operations: vec![CapabilityOperation::Observe as i32],
                    requested_access_class: CapabilityAccessClass::Derived as i32,
                    requested_constraints: Vec::new(),
                    correlation_id: Vec::new(),
                },
            )),
        })?;

        // Receive accept
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
