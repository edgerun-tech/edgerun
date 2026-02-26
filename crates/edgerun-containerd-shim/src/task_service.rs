// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{ContainerdTaskClient, TaskApiResponse};

#[derive(Debug, Clone)]
pub struct TaskCreateRequest {
    pub namespace: String,
    pub task_id: String,
    pub runtime_name: Option<String>,
    pub runtime_selector_source: Option<String>,
    pub bundle_path: Option<String>,
    pub stdin_path: Option<String>,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    pub rootfs_source: Option<String>,
    pub rootfs_readonly: Option<bool>,
    pub rootfs_type: Option<String>,
    pub rootfs_options_csv: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskStartRequest {
    pub namespace: String,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskStateRequest {
    pub namespace: String,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskKillRequest {
    pub namespace: String,
    pub task_id: String,
    pub signal: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TaskDeleteRequest {
    pub namespace: String,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskWaitRequest {
    pub namespace: String,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskRpcResponse {
    pub state: String,
    pub pid: Option<u32>,
    pub exit_code: Option<u32>,
    pub pending: bool,
    pub message: String,
}

#[async_trait]
pub trait TaskApiBackend: Send + Sync {
    async fn create(&mut self, req: &TaskCreateRequest) -> Result<TaskApiResponse>;
    async fn start(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse>;
    async fn state(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse>;
    async fn kill(
        &mut self,
        namespace: &str,
        task_id: &str,
        signal: Option<u32>,
    ) -> Result<TaskApiResponse>;
    async fn delete(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse>;
    async fn wait(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse>;
}

#[async_trait]
impl TaskApiBackend for ContainerdTaskClient {
    async fn create(&mut self, req: &TaskCreateRequest) -> Result<TaskApiResponse> {
        ContainerdTaskClient::create(self, req).await
    }

    async fn start(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        ContainerdTaskClient::start(self, namespace, task_id).await
    }

    async fn state(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        ContainerdTaskClient::state(self, namespace, task_id).await
    }

    async fn kill(
        &mut self,
        namespace: &str,
        task_id: &str,
        signal: Option<u32>,
    ) -> Result<TaskApiResponse> {
        ContainerdTaskClient::kill(self, namespace, task_id, signal).await
    }

    async fn delete(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        ContainerdTaskClient::delete(self, namespace, task_id).await
    }

    async fn wait(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        ContainerdTaskClient::wait(self, namespace, task_id).await
    }
}

#[derive(Clone)]
pub struct ShimTaskTtrpcService<B: TaskApiBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: TaskApiBackend> ShimTaskTtrpcService<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(Mutex::new(backend)),
        }
    }

    pub async fn create(&self, req: TaskCreateRequest) -> Result<TaskRpcResponse> {
        let mut backend = self.backend.lock().await;
        let out = backend.create(&req).await?;
        Ok(map_response(out))
    }

    pub async fn start(&self, req: TaskStartRequest) -> Result<TaskRpcResponse> {
        let mut backend = self.backend.lock().await;
        let out = backend.start(&req.namespace, &req.task_id).await?;
        Ok(map_response(out))
    }

    pub async fn state(&self, req: TaskStateRequest) -> Result<TaskRpcResponse> {
        let mut backend = self.backend.lock().await;
        let out = backend.state(&req.namespace, &req.task_id).await?;
        Ok(map_response(out))
    }

    pub async fn kill(&self, req: TaskKillRequest) -> Result<TaskRpcResponse> {
        let mut backend = self.backend.lock().await;
        let out = backend
            .kill(&req.namespace, &req.task_id, req.signal)
            .await?;
        Ok(map_response(out))
    }

    pub async fn delete(&self, req: TaskDeleteRequest) -> Result<TaskRpcResponse> {
        let mut backend = self.backend.lock().await;
        let out = backend.delete(&req.namespace, &req.task_id).await?;
        Ok(map_response(out))
    }

    pub async fn wait(&self, req: TaskWaitRequest) -> Result<TaskRpcResponse> {
        let mut backend = self.backend.lock().await;
        let out = backend.wait(&req.namespace, &req.task_id).await?;
        Ok(map_response(out))
    }
}

fn map_response(resp: TaskApiResponse) -> TaskRpcResponse {
    TaskRpcResponse {
        state: resp
            .state
            .map(|s| format!("{s:?}").to_lowercase())
            .unwrap_or_default(),
        pid: resp.pid,
        exit_code: resp.exit_code,
        pending: resp.pending,
        message: resp.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskLifecycleState;

    #[derive(Default)]
    struct MockBackend {
        pub calls: Vec<String>,
    }

    #[async_trait]
    impl TaskApiBackend for MockBackend {
        async fn create(&mut self, req: &TaskCreateRequest) -> Result<TaskApiResponse> {
            self.calls.push(format!(
                "create:{}:{}:{}:{}",
                req.namespace,
                req.task_id,
                req.runtime_name.clone().unwrap_or_default(),
                req.runtime_selector_source.clone().unwrap_or_default(),
            ));
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Created),
                pid: None,
                exit_code: None,
                pending: false,
                message: "created".to_string(),
                runtime_name: req.runtime_name.clone(),
            })
        }

        async fn start(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
            self.calls.push(format!("start:{namespace}:{task_id}"));
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Running),
                pid: Some(100),
                exit_code: None,
                pending: false,
                message: "started".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn state(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
            self.calls.push(format!("state:{namespace}:{task_id}"));
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Running),
                pid: Some(100),
                exit_code: None,
                pending: false,
                message: "state".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn kill(
            &mut self,
            namespace: &str,
            task_id: &str,
            signal: Option<u32>,
        ) -> Result<TaskApiResponse> {
            self.calls
                .push(format!("kill:{namespace}:{task_id}:{:?}", signal));
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Exited),
                pid: Some(100),
                exit_code: signal.or(Some(137)),
                pending: false,
                message: "killed".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn delete(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
            self.calls.push(format!("delete:{namespace}:{task_id}"));
            Ok(TaskApiResponse {
                state: None,
                pid: None,
                exit_code: None,
                pending: false,
                message: "deleted".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }

        async fn wait(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
            self.calls.push(format!("wait:{namespace}:{task_id}"));
            Ok(TaskApiResponse {
                state: Some(TaskLifecycleState::Exited),
                pid: Some(100),
                exit_code: Some(0),
                pending: false,
                message: "completed".to_string(),
                runtime_name: Some("edgerun".to_string()),
            })
        }
    }

    #[tokio::test]
    async fn task_rpc_methods_delegate_to_backend() {
        let service = ShimTaskTtrpcService::new(MockBackend::default());

        let c = service
            .create(TaskCreateRequest {
                namespace: "ns".to_string(),
                task_id: "t1".to_string(),
                runtime_name: Some("edgerun".to_string()),
                runtime_selector_source: Some("annotation:io.edgerun.runtime".to_string()),
                bundle_path: Some("/tmp/bundle".to_string()),
                stdin_path: Some("/tmp/stdin".to_string()),
                stdout_path: Some("/tmp/stdout".to_string()),
                stderr_path: Some("/tmp/stderr".to_string()),
                rootfs_source: Some("/tmp/rootfs".to_string()),
                rootfs_readonly: Some(true),
                rootfs_type: Some("bind".to_string()),
                rootfs_options_csv: Some("ro,rbind".to_string()),
            })
            .await
            .expect("create");
        assert_eq!(c.state, "created");

        let s = service
            .start(TaskStartRequest {
                namespace: "ns".to_string(),
                task_id: "t1".to_string(),
            })
            .await
            .expect("start");
        assert_eq!(s.state, "running");
        assert_eq!(s.pid, Some(100));

        let k = service
            .kill(TaskKillRequest {
                namespace: "ns".to_string(),
                task_id: "t1".to_string(),
                signal: Some(9),
            })
            .await
            .expect("kill");
        assert_eq!(k.exit_code, Some(9));

        let d = service
            .delete(TaskDeleteRequest {
                namespace: "ns".to_string(),
                task_id: "t1".to_string(),
            })
            .await
            .expect("delete");
        assert_eq!(d.message, "deleted");
    }
}
