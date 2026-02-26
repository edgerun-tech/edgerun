// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use edgerun_runtime_proto::wire::{
    shim_request_v1, ShimRequestV1, ShimResponseV1, TaskServiceOpV1, TaskServiceRequestV1,
};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::{TaskApiResponse, TaskCreateRequest, TaskLifecycleState};

#[derive(Debug, thiserror::Error)]
pub enum TaskApiClientError {
    #[error("unsupported schema_version in response: {0}")]
    UnsupportedSchema(u32),
    #[error("remote task service error: {0}")]
    RemoteError(String),
    #[error("invalid task state in response: {0}")]
    InvalidState(String),
}

#[derive(Debug)]
pub struct ContainerdTaskClient {
    stream: UnixStream,
    schema_version: u32,
}

impl ContainerdTaskClient {
    pub async fn connect(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let stream = UnixStream::connect(path.as_ref())
            .await
            .with_context(|| format!("connect shim socket {}", path.as_ref().display()))?;
        Ok(Self {
            stream,
            schema_version: 1,
        })
    }

    pub async fn create(&mut self, req: &TaskCreateRequest) -> Result<TaskApiResponse> {
        self.call_task_api(req, TaskServiceOpV1::Create, None).await
    }

    pub async fn start(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        self.call_task_api(
            &TaskCreateRequest {
                namespace: namespace.to_string(),
                task_id: task_id.to_string(),
                runtime_name: None,
                runtime_selector_source: None,
                bundle_path: None,
                stdin_path: None,
                stdout_path: None,
                stderr_path: None,
                rootfs_source: None,
                rootfs_readonly: None,
                rootfs_type: None,
                rootfs_options_csv: None,
            },
            TaskServiceOpV1::Start,
            None,
        )
        .await
    }

    pub async fn state(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        self.call_task_api(
            &TaskCreateRequest {
                namespace: namespace.to_string(),
                task_id: task_id.to_string(),
                runtime_name: None,
                runtime_selector_source: None,
                bundle_path: None,
                stdin_path: None,
                stdout_path: None,
                stderr_path: None,
                rootfs_source: None,
                rootfs_readonly: None,
                rootfs_type: None,
                rootfs_options_csv: None,
            },
            TaskServiceOpV1::State,
            None,
        )
        .await
    }

    pub async fn kill(
        &mut self,
        namespace: &str,
        task_id: &str,
        signal: Option<u32>,
    ) -> Result<TaskApiResponse> {
        self.call_task_api(
            &TaskCreateRequest {
                namespace: namespace.to_string(),
                task_id: task_id.to_string(),
                runtime_name: None,
                runtime_selector_source: None,
                bundle_path: None,
                stdin_path: None,
                stdout_path: None,
                stderr_path: None,
                rootfs_source: None,
                rootfs_readonly: None,
                rootfs_type: None,
                rootfs_options_csv: None,
            },
            TaskServiceOpV1::Kill,
            signal,
        )
        .await
    }

    pub async fn delete(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        self.call_task_api(
            &TaskCreateRequest {
                namespace: namespace.to_string(),
                task_id: task_id.to_string(),
                runtime_name: None,
                runtime_selector_source: None,
                bundle_path: None,
                stdin_path: None,
                stdout_path: None,
                stderr_path: None,
                rootfs_source: None,
                rootfs_readonly: None,
                rootfs_type: None,
                rootfs_options_csv: None,
            },
            TaskServiceOpV1::Delete,
            None,
        )
        .await
    }

    pub async fn wait(&mut self, namespace: &str, task_id: &str) -> Result<TaskApiResponse> {
        self.call_task_api(
            &TaskCreateRequest {
                namespace: namespace.to_string(),
                task_id: task_id.to_string(),
                runtime_name: None,
                runtime_selector_source: None,
                bundle_path: None,
                stdin_path: None,
                stdout_path: None,
                stderr_path: None,
                rootfs_source: None,
                rootfs_readonly: None,
                rootfs_type: None,
                rootfs_options_csv: None,
            },
            TaskServiceOpV1::Wait,
            None,
        )
        .await
    }

    async fn call_task_api(
        &mut self,
        req: &TaskCreateRequest,
        op: TaskServiceOpV1,
        signal: Option<u32>,
    ) -> Result<TaskApiResponse> {
        let wire_req = build_task_api_request(self.schema_version, req, op, signal);
        let resp = self.call(wire_req).await?;
        decode_task_api_response(resp).map_err(Into::into)
    }

    async fn call(&mut self, req: ShimRequestV1) -> Result<ShimResponseV1> {
        write_pb_frame(&mut self.stream, &req).await?;
        read_pb_frame(&mut self.stream).await
    }
}

pub fn build_task_api_request(
    schema_version: u32,
    req: &TaskCreateRequest,
    op: TaskServiceOpV1,
    signal: Option<u32>,
) -> ShimRequestV1 {
    ShimRequestV1 {
        schema_version,
        op: Some(shim_request_v1::Op::TaskApi(TaskServiceRequestV1 {
            namespace: req.namespace.clone(),
            task_id: req.task_id.clone(),
            op: op as i32,
            signal,
            runtime_name: req.runtime_name.clone().unwrap_or_default(),
            runtime_selector_source: req.runtime_selector_source.clone().unwrap_or_default(),
            bundle_path: req.bundle_path.clone().unwrap_or_default(),
            stdin_path: req.stdin_path.clone().unwrap_or_default(),
            stdout_path: req.stdout_path.clone().unwrap_or_default(),
            stderr_path: req.stderr_path.clone().unwrap_or_default(),
            rootfs_source: req.rootfs_source.clone().unwrap_or_default(),
            rootfs_readonly: req.rootfs_readonly.unwrap_or(false),
            rootfs_type: req.rootfs_type.clone().unwrap_or_default(),
            rootfs_options_csv: req.rootfs_options_csv.clone().unwrap_or_default(),
        })),
    }
}

pub fn decode_task_api_response(
    resp: ShimResponseV1,
) -> std::result::Result<TaskApiResponse, TaskApiClientError> {
    if resp.schema_version != 1 {
        return Err(TaskApiClientError::UnsupportedSchema(resp.schema_version));
    }
    if !resp.ok {
        return Err(TaskApiClientError::RemoteError(resp.message));
    }

    let state = if resp.state.is_empty() {
        None
    } else {
        Some(parse_state(&resp.state)?)
    };

    Ok(TaskApiResponse {
        state,
        pid: resp.pid,
        exit_code: resp.exit_code,
        pending: resp.pending,
        message: resp.message,
        runtime_name: None,
    })
}

fn parse_state(value: &str) -> std::result::Result<TaskLifecycleState, TaskApiClientError> {
    match value {
        "created" => Ok(TaskLifecycleState::Created),
        "running" => Ok(TaskLifecycleState::Running),
        "exited" => Ok(TaskLifecycleState::Exited),
        other => Err(TaskApiClientError::InvalidState(other.to_string())),
    }
}

async fn read_pb_frame<T: Message + Default>(stream: &mut UnixStream) -> Result<T> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .context("read frame length")?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    stream
        .read_exact(&mut payload)
        .await
        .context("read frame payload")?;
    T::decode(payload.as_slice()).context("decode protobuf frame")
}

async fn write_pb_frame<T: Message>(stream: &mut UnixStream, value: &T) -> Result<()> {
    let mut payload = Vec::new();
    value
        .encode(&mut payload)
        .context("encode protobuf frame")?;
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len).await.context("write frame length")?;
    stream
        .write_all(&payload)
        .await
        .context("write frame payload")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_maps_task_api_op() {
        let req = build_task_api_request(
            1,
            &TaskCreateRequest {
                namespace: "ns".to_string(),
                task_id: "task-1".to_string(),
                runtime_name: Some("edgerun".to_string()),
                runtime_selector_source: Some("annotation:io.edgerun.runtime".to_string()),
                bundle_path: Some("/tmp/demo-bundle".to_string()),
                stdin_path: Some("/tmp/demo-stdin".to_string()),
                stdout_path: Some("/tmp/demo-stdout".to_string()),
                stderr_path: Some("/tmp/demo-stderr".to_string()),
                rootfs_source: Some("/tmp/demo-rootfs".to_string()),
                rootfs_readonly: Some(true),
                rootfs_type: Some("overlay".to_string()),
                rootfs_options_csv: Some("lowerdir=/a,upperdir=/b,workdir=/c".to_string()),
            },
            TaskServiceOpV1::Start,
            None,
        );
        assert_eq!(req.schema_version, 1);
        match req.op {
            Some(shim_request_v1::Op::TaskApi(v)) => {
                assert_eq!(v.namespace, "ns");
                assert_eq!(v.task_id, "task-1");
                assert_eq!(v.op, TaskServiceOpV1::Start as i32);
                assert_eq!(v.runtime_name, "edgerun");
                assert_eq!(v.runtime_selector_source, "annotation:io.edgerun.runtime");
                assert_eq!(v.bundle_path, "/tmp/demo-bundle");
                assert_eq!(v.stdin_path, "/tmp/demo-stdin");
                assert_eq!(v.stdout_path, "/tmp/demo-stdout");
                assert_eq!(v.stderr_path, "/tmp/demo-stderr");
                assert_eq!(v.rootfs_source, "/tmp/demo-rootfs");
                assert!(v.rootfs_readonly);
                assert_eq!(v.rootfs_type, "overlay");
                assert_eq!(v.rootfs_options_csv, "lowerdir=/a,upperdir=/b,workdir=/c");
            }
            _ => panic!("expected task_api op"),
        }
    }

    #[test]
    fn decode_response_maps_task_fields() {
        let out = decode_task_api_response(ShimResponseV1 {
            schema_version: 1,
            ok: true,
            message: "started".to_string(),
            offset: Some(42),
            subject: String::new(),
            state: "running".to_string(),
            pending: false,
            pid: Some(123),
            exit_code: None,
        })
        .expect("decode");
        assert_eq!(out.state, Some(TaskLifecycleState::Running));
        assert_eq!(out.pid, Some(123));
        assert!(!out.pending);
        assert_eq!(out.message, "started");
    }

    #[test]
    fn decode_response_rejects_invalid_state() {
        let err = decode_task_api_response(ShimResponseV1 {
            schema_version: 1,
            ok: true,
            message: "bad".to_string(),
            offset: None,
            subject: String::new(),
            state: "weird".to_string(),
            pending: false,
            pid: None,
            exit_code: None,
        })
        .expect_err("must fail");
        assert!(matches!(err, TaskApiClientError::InvalidState(_)));
    }
}
