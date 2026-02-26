// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use containerd_shim::{
    asynchronous::{run, spawn, ExitSignal, Shim},
    publisher::RemotePublisher,
    Config, Error, Flags, StartOpts,
};
use containerd_shim_protos::{
    api::DeleteResponse,
    protobuf::{Message, MessageField},
    types::introspection::{RuntimeInfo, RuntimeVersion},
};
use edgerun_containerd_shim::{
    ContainerdTaskClient, ContainerdTaskTtrpcService, ShimTaskTtrpcService,
};

const RUNTIME_ID: &str = "io.containerd.edgerun.v1";
const DEFAULT_BACKEND_SOCKET: &str = "/run/edgerun-shim/edgerun.sock";
const ENV_BACKEND_SOCKET: &str = "EDGERUN_SHIM_SOCKET";

#[derive(Clone)]
struct EdgeRunShim {
    exit: Arc<ExitSignal>,
    backend_socket: PathBuf,
}

#[async_trait]
impl Shim for EdgeRunShim {
    type T = ContainerdTaskTtrpcService<ContainerdTaskClient>;

    async fn new(_runtime_id: &str, _args: &Flags, _config: &mut Config) -> Self {
        let backend_socket = env::var(ENV_BACKEND_SOCKET)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_BACKEND_SOCKET));
        Self {
            exit: Arc::new(ExitSignal::default()),
            backend_socket,
        }
    }

    async fn start_shim(&mut self, opts: StartOpts) -> Result<String, Error> {
        let grouping = opts.id.clone();
        spawn(opts, &grouping, Vec::new()).await
    }

    async fn delete_shim(&mut self) -> Result<DeleteResponse, Error> {
        Ok(DeleteResponse::new())
    }

    async fn wait(&mut self) {
        self.exit.wait().await;
    }

    async fn create_task_service(&self, _publisher: RemotePublisher) -> Self::T {
        let client = ContainerdTaskClient::connect(&self.backend_socket)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "failed to connect {}={} ({err:#})",
                    ENV_BACKEND_SOCKET,
                    self.backend_socket.display()
                )
            });

        let exit = self.exit.clone();
        ContainerdTaskTtrpcService::new(ShimTaskTtrpcService::new(client))
            .with_runtime_version("edgerun.v1")
            .with_shutdown_hook(move || exit.signal())
    }
}

#[tokio::main]
async fn main() {
    if env::args().any(|arg| arg == "-info") {
        let info = RuntimeInfo {
            name: "containerd-shim-edgerun-v2".to_string(),
            version: MessageField::some(RuntimeVersion {
                version: env!("CARGO_PKG_VERSION").to_string(),
                revision: String::new(),
                ..Default::default()
            }),
            ..Default::default()
        };
        match info.write_to_bytes() {
            Ok(bytes) => {
                if let Err(err) = std::io::stdout().write_all(&bytes) {
                    eprintln!("{RUNTIME_ID}: {err:#}");
                    std::process::exit(1);
                }
            }
            Err(err) => {
                eprintln!("{RUNTIME_ID}: {err:#}");
                std::process::exit(1);
            }
        }
        return;
    }
    run::<EdgeRunShim>(RUNTIME_ID, None).await;
}
