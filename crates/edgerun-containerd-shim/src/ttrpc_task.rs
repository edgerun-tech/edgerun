// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use containerd_shim_protos::api::{
    ConnectRequest, ConnectResponse, CreateTaskRequest, CreateTaskResponse, DeleteRequest,
    DeleteResponse, Empty, KillRequest, ShutdownRequest, StartRequest, StartResponse, StateRequest,
    StateResponse, WaitRequest, WaitResponse,
};
use containerd_shim_protos::shim_async::{create_task, Task};
use containerd_shim_protos::ttrpc::asynchronous::{Server, Service, TtrpcContext};
use serde_json::Value;

use crate::{
    ShimTaskTtrpcService, TaskApiBackend, TaskCreateRequest, TaskDeleteRequest, TaskKillRequest,
    TaskStartRequest, TaskStateRequest, TaskWaitRequest,
};

#[derive(Clone)]
pub struct ContainerdTaskTtrpcService<B: TaskApiBackend + 'static> {
    inner: ShimTaskTtrpcService<B>,
    runtime_version: String,
    shutdown_hook: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl<B: TaskApiBackend + 'static> ContainerdTaskTtrpcService<B> {
    pub fn new(inner: ShimTaskTtrpcService<B>) -> Self {
        Self {
            inner,
            runtime_version: "edgerun.v1".to_string(),
            shutdown_hook: None,
        }
    }

    pub fn with_runtime_version(mut self, version: impl Into<String>) -> Self {
        self.runtime_version = version.into();
        self
    }

    pub fn with_shutdown_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.shutdown_hook = Some(Arc::new(hook));
        self
    }

    pub fn into_registered_service(self) -> HashMap<String, Service> {
        create_task(Arc::new(self))
    }
}

pub fn register_task_ttrpc_service<B: TaskApiBackend + 'static>(
    server: Server,
    service: ContainerdTaskTtrpcService<B>,
) -> Server {
    server.register_service(service.into_registered_service())
}

#[async_trait]
impl<B: TaskApiBackend + 'static> Task for ContainerdTaskTtrpcService<B> {
    async fn create(
        &self,
        ctx: &TtrpcContext,
        req: CreateTaskRequest,
    ) -> containerd_shim_protos::ttrpc::Result<CreateTaskResponse> {
        let namespace = namespace_from_ctx(ctx);
        let selected = runtime_selection_from_bundle(&req.bundle);
        let (rootfs_source, rootfs_readonly, rootfs_type, rootfs_options_csv) =
            primary_rootfs_from_create(&req);
        let out = self
            .inner
            .create(TaskCreateRequest {
                namespace,
                task_id: req.id.clone(),
                runtime_name: selected.runtime_name,
                runtime_selector_source: selected.selector_source,
                bundle_path: Some(req.bundle.clone()),
                stdin_path: Some(req.stdin.clone()),
                stdout_path: Some(req.stdout.clone()),
                stderr_path: Some(req.stderr.clone()),
                rootfs_source,
                rootfs_readonly,
                rootfs_type,
                rootfs_options_csv,
            })
            .await
            .map_err(to_ttrpc_error)?;

        let mut resp = CreateTaskResponse::new();
        resp.set_pid(out.pid.unwrap_or_default());
        Ok(resp)
    }

    async fn start(
        &self,
        ctx: &TtrpcContext,
        req: StartRequest,
    ) -> containerd_shim_protos::ttrpc::Result<StartResponse> {
        let namespace = namespace_from_ctx(ctx);
        let task_id = select_task_id(&req.id, &req.exec_id);
        let out = self
            .inner
            .start(TaskStartRequest { namespace, task_id })
            .await
            .map_err(to_ttrpc_error)?;

        let mut resp = StartResponse::new();
        resp.set_pid(out.pid.unwrap_or_default());
        Ok(resp)
    }

    async fn state(
        &self,
        ctx: &TtrpcContext,
        req: StateRequest,
    ) -> containerd_shim_protos::ttrpc::Result<StateResponse> {
        let namespace = namespace_from_ctx(ctx);
        let task_id = select_task_id(&req.id, &req.exec_id);
        let out = self
            .inner
            .state(TaskStateRequest {
                namespace,
                task_id: task_id.clone(),
            })
            .await
            .map_err(to_ttrpc_error)?;

        let mut resp = StateResponse::new();
        resp.set_id(req.id);
        resp.set_exec_id(req.exec_id);
        resp.set_pid(out.pid.unwrap_or_default());
        resp.set_status(map_status(&out.state));
        resp.set_exit_status(out.exit_code.unwrap_or_default());
        Ok(resp)
    }

    async fn kill(
        &self,
        ctx: &TtrpcContext,
        req: KillRequest,
    ) -> containerd_shim_protos::ttrpc::Result<Empty> {
        let namespace = namespace_from_ctx(ctx);
        let task_id = select_task_id(&req.id, &req.exec_id);
        self.inner
            .kill(TaskKillRequest {
                namespace,
                task_id,
                signal: Some(req.signal),
            })
            .await
            .map_err(to_ttrpc_error)?;
        Ok(Empty::new())
    }

    async fn delete(
        &self,
        ctx: &TtrpcContext,
        req: DeleteRequest,
    ) -> containerd_shim_protos::ttrpc::Result<DeleteResponse> {
        let namespace = namespace_from_ctx(ctx);
        let task_id = select_task_id(&req.id, &req.exec_id);
        let out = self
            .inner
            .delete(TaskDeleteRequest { namespace, task_id })
            .await
            .map_err(to_ttrpc_error)?;

        let mut resp = DeleteResponse::new();
        resp.set_pid(out.pid.unwrap_or_default());
        resp.set_exit_status(out.exit_code.unwrap_or_default());
        Ok(resp)
    }

    async fn wait(
        &self,
        ctx: &TtrpcContext,
        req: WaitRequest,
    ) -> containerd_shim_protos::ttrpc::Result<WaitResponse> {
        let namespace = namespace_from_ctx(ctx);
        let task_id = select_task_id(&req.id, &req.exec_id);
        let out = self
            .inner
            .wait(TaskWaitRequest { namespace, task_id })
            .await
            .map_err(to_ttrpc_error)?;

        let mut resp = WaitResponse::new();
        resp.set_exit_status(out.exit_code.unwrap_or_default());
        Ok(resp)
    }

    async fn connect(
        &self,
        _ctx: &TtrpcContext,
        _req: ConnectRequest,
    ) -> containerd_shim_protos::ttrpc::Result<ConnectResponse> {
        let mut resp = ConnectResponse::new();
        resp.set_shim_pid(std::process::id());
        resp.set_version(self.runtime_version.clone());
        Ok(resp)
    }

    async fn shutdown(
        &self,
        _ctx: &TtrpcContext,
        _req: ShutdownRequest,
    ) -> containerd_shim_protos::ttrpc::Result<Empty> {
        if let Some(hook) = &self.shutdown_hook {
            hook();
        }
        Ok(Empty::new())
    }
}

fn namespace_from_ctx(ctx: &TtrpcContext) -> String {
    ctx.metadata
        .get("containerd-namespace")
        .and_then(|vals| vals.first())
        .cloned()
        .unwrap_or_else(|| "default".to_string())
}

fn select_task_id(id: &str, exec_id: &str) -> String {
    if exec_id.is_empty() {
        id.to_string()
    } else {
        exec_id.to_string()
    }
}

fn map_status(value: &str) -> containerd_shim_protos::api::Status {
    match value {
        "created" => containerd_shim_protos::api::Status::CREATED,
        "running" => containerd_shim_protos::api::Status::RUNNING,
        "exited" => containerd_shim_protos::api::Status::STOPPED,
        _ => containerd_shim_protos::api::Status::UNKNOWN,
    }
}

fn to_ttrpc_error(err: anyhow::Error) -> containerd_shim_protos::ttrpc::Error {
    containerd_shim_protos::ttrpc::Error::Others(err.to_string())
}

#[derive(Debug, Clone, Default)]
struct RuntimeSelection {
    runtime_name: Option<String>,
    selector_source: Option<String>,
}

fn runtime_selection_from_bundle(bundle_path: &str) -> RuntimeSelection {
    if bundle_path.trim().is_empty() {
        return RuntimeSelection::default();
    }
    let config_path = Path::new(bundle_path).join("config.json");
    let raw = match std::fs::read(&config_path) {
        Ok(v) => v,
        Err(_) => return RuntimeSelection::default(),
    };
    let parsed: Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(_) => return RuntimeSelection::default(),
    };
    let annotations = match parsed.get("annotations").and_then(|v| v.as_object()) {
        Some(v) => v,
        None => return RuntimeSelection::default(),
    };

    for key in [
        "io.edgerun.runtime.class",
        "io.edgerun.runtime",
        "io.edgerun.executor",
    ] {
        let Some(raw_value) = annotations.get(key).and_then(|v| v.as_str()) else {
            continue;
        };
        let value = raw_value.trim().to_ascii_lowercase();
        if value.is_empty() {
            continue;
        }
        if matches!(value.as_str(), "edgerun" | "wasi" | "wasm" | "edgerun-wasi") {
            return RuntimeSelection {
                runtime_name: Some("edgerun".to_string()),
                selector_source: Some(format!("annotation:{key}={value}")),
            };
        }
        if matches!(value.as_str(), "crun" | "oci" | "runc") {
            return RuntimeSelection {
                runtime_name: Some("crun".to_string()),
                selector_source: Some(format!("annotation:{key}={value}")),
            };
        }
        return RuntimeSelection {
            runtime_name: Some(value.clone()),
            selector_source: Some(format!("annotation:{key}={value}")),
        };
    }

    RuntimeSelection::default()
}

fn primary_rootfs_from_create(
    req: &CreateTaskRequest,
) -> (Option<String>, Option<bool>, Option<String>, Option<String>) {
    let mount = req.rootfs.first();
    let Some(mount) = mount else {
        return (None, None, None, None);
    };
    if mount.source.trim().is_empty() && mount.type_.trim().is_empty() {
        return (None, None, None, None);
    }
    let readonly = mount.options.iter().any(|v| v.eq_ignore_ascii_case("ro"));
    (
        Some(mount.source.clone()),
        Some(readonly),
        Some(mount.type_.clone()),
        Some(mount.options.join(",")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TaskApiResponse, TaskLifecycleState};
    use containerd_shim_protos::ttrpc::proto::MessageHeader;

    #[derive(Default)]
    struct MockBackend;

    #[async_trait]
    impl TaskApiBackend for MockBackend {
        async fn create(&mut self, req: &TaskCreateRequest) -> anyhow::Result<TaskApiResponse> {
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Created),
                pid: Some(101),
                exit_code: None,
                pending: false,
                message: format!(
                    "created:{}:{}",
                    req.task_id,
                    req.runtime_name.clone().unwrap_or_default()
                ),
                runtime_name: req.runtime_name.clone(),
            })
        }

        async fn start(
            &mut self,
            _namespace: &str,
            task_id: &str,
        ) -> anyhow::Result<TaskApiResponse> {
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Running),
                pid: Some(202),
                exit_code: None,
                pending: false,
                message: format!("started:{task_id}"),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn state(
            &mut self,
            _namespace: &str,
            _task_id: &str,
        ) -> anyhow::Result<TaskApiResponse> {
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Running),
                pid: Some(202),
                exit_code: None,
                pending: false,
                message: "state".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn kill(
            &mut self,
            _namespace: &str,
            _task_id: &str,
            signal: Option<u32>,
        ) -> anyhow::Result<TaskApiResponse> {
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Exited),
                pid: Some(202),
                exit_code: signal,
                pending: false,
                message: "killed".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn delete(
            &mut self,
            _namespace: &str,
            _task_id: &str,
        ) -> anyhow::Result<TaskApiResponse> {
            Ok(TaskApiResponse {
                state: None,
                pid: Some(202),
                exit_code: Some(0),
                pending: false,
                message: "deleted".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn wait(
            &mut self,
            _namespace: &str,
            _task_id: &str,
        ) -> anyhow::Result<TaskApiResponse> {
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Exited),
                pid: Some(202),
                exit_code: Some(0),
                pending: false,
                message: "completed".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }
    }

    #[tokio::test]
    async fn ttrpc_task_methods_map_to_runtime_service() {
        let svc = ContainerdTaskTtrpcService::new(ShimTaskTtrpcService::new(MockBackend));
        let ctx = TtrpcContext {
            fd: 0,
            mh: MessageHeader::default(),
            metadata: HashMap::new(),
            timeout_nano: 0,
        };

        let mut create_req = CreateTaskRequest::new();
        create_req.set_id("task-1".to_string());
        create_req.set_bundle("/tmp/no-bundle".to_string());
        let create = Task::create(&svc, &ctx, create_req).await.expect("create");
        assert_eq!(create.pid, 101);

        let mut start_req = StartRequest::new();
        start_req.set_id("task-1".to_string());
        let start = Task::start(&svc, &ctx, start_req).await.expect("start");
        assert_eq!(start.pid, 202);

        let mut state_req = StateRequest::new();
        state_req.set_id("task-1".to_string());
        let state = Task::state(&svc, &ctx, state_req).await.expect("state");
        assert_eq!(
            state.status.enum_value_or_default(),
            containerd_shim_protos::api::Status::RUNNING
        );

        let mut kill_req = KillRequest::new();
        kill_req.set_id("task-1".to_string());
        kill_req.set_signal(9);
        Task::kill(&svc, &ctx, kill_req).await.expect("kill");

        let mut wait_req = WaitRequest::new();
        wait_req.set_id("task-1".to_string());
        let wait = Task::wait(&svc, &ctx, wait_req).await.expect("wait");
        assert_eq!(wait.exit_status, 0);

        let mut del_req = DeleteRequest::new();
        del_req.set_id("task-1".to_string());
        let del = Task::delete(&svc, &ctx, del_req).await.expect("delete");
        assert_eq!(del.exit_status, 0);
    }

    #[test]
    fn runtime_selection_prefers_known_annotations() {
        let dir = std::env::temp_dir().join(format!("edgerun-shim-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let config = r#"{
            "ociVersion":"1.0.2",
            "annotations": {
                "io.edgerun.executor":"wasi"
            }
        }"#;
        std::fs::write(dir.join("config.json"), config).expect("write config");
        let selected = runtime_selection_from_bundle(&dir.to_string_lossy());
        assert_eq!(selected.runtime_name.as_deref(), Some("edgerun"));
        assert_eq!(
            selected.selector_source.as_deref(),
            Some("annotation:io.edgerun.executor=wasi")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
