//! Unified DNS + DHCP server.
//!
//! One process replaces:
//! - dnsmasq (DNS forwarder + DHCP server)
//! - BIND/PowerDNS (authoritative DNS)
//! - CoreDNS (DNS forwarding + plugins)
//! - ISC DHCP/Kea (DHCPv4 + DHCPv6)
//! - TFTP servers (PXE boot)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                edgerun-net                       │
//! │                                                 │
//! │  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
//! │  │ DNS      │  │ DHCPv4   │  │ DHCPv6        │  │
//! │  │ Server   │  │ Server   │  │ Server        │  │
//! │  │ :53      │  │ :67      │  │ :547          │  │
//! │  └────┬─────┘  └────┬─────┘  └───────┬───────┘  │
//! │       │              │                │          │
//! │       └──────────────┼────────────────┘          │
//! │                      │                           │
//! │              ┌───────▼───────┐                   │
//! │              │  Integration  │                   │
//! │              │  Layer        │                   │
//! │              │  (A/PTR sync) │                   │
//! │              └───────┬───────┘                   │
//! │                      │                           │
//! │              ┌───────▼───────┐                   │
//! │              │  Config       │                   │
//! │              │  (K8s YAML)   │                   │
//! │              │  (hot-reload) │                   │
//! │              └───────────────┘                   │
//! └─────────────────────────────────────────────────┘
//! ```

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(not(target_os = "none"))]
mod config_watch;
pub mod integration;
#[cfg(not(target_os = "none"))]
pub mod server;

#[cfg(target_os = "none")]
mod config_watch {
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicBool, Ordering};

    pub struct ConfigWatcher {
        changed: AtomicBool,
        stopped: AtomicBool,
    }

    impl ConfigWatcher {
        pub fn new<P>(_paths: &[P]) -> Self {
            Self {
                changed: AtomicBool::new(false),
                stopped: AtomicBool::new(false),
            }
        }

        pub fn take_changed(&self) -> bool {
            self.changed.swap(false, Ordering::SeqCst)
        }

        pub fn is_stopped(&self) -> bool {
            self.stopped.load(Ordering::SeqCst)
        }

        pub fn start(&self) -> Result<Self, edgerun_config::ConfigError> {
            Ok(Self::new::<&str>(&[]))
        }

        pub fn stop(&self) {
            self.stopped.store(true, Ordering::SeqCst);
        }
    }

    impl Drop for ConfigWatcher {
        fn drop(&mut self) {
            self.stop();
        }
    }
}

#[cfg(target_os = "none")]
mod server {
    use alloc::boxed::Box;
    use alloc::string::{String, ToString};
    use core::fmt;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NetServerUnavailable;

    impl fmt::Display for NetServerUnavailable {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("edgerun-net hosted daemon is unavailable on bare target")
        }
    }

    impl core::error::Error for NetServerUnavailable {}

    pub struct NetServer {
        config_path: String,
        hot_reload: bool,
    }

    impl NetServer {
        pub fn new(config_path: &str, hot_reload: bool) -> Self {
            Self {
                config_path: config_path.to_string(),
                hot_reload,
            }
        }

        pub fn run(&self) -> Result<(), Box<dyn core::error::Error>> {
            let _ = (&self.config_path, self.hot_reload);
            Err(Box::new(NetServerUnavailable))
        }
    }
}

pub use config_watch::ConfigWatcher;
pub use integration::DnsDhcpIntegration;
pub use server::NetServer;
