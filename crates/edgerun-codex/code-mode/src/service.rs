use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use edgerun_async_trait::async_trait;
use edgerun_json::serde_json::Value as JsonValue;
use edgerun_tokio::sync::Mutex;
use edgerun_tokio::sync::oneshot;
use edgerun_tokio_util::sync::CancellationToken;

use crate::runtime::CodeModeNestedToolCall;
use crate::runtime::ExecuteRequest;
use crate::runtime::RuntimeResponse;
use crate::runtime::WaitOutcome;
use crate::runtime::WaitRequest;
use crate::runtime::missing_cell_response;

#[async_trait]
pub trait CodeModeTurnHost: Send + Sync {
    async fn invoke_tool(
        &self,
        invocation: CodeModeNestedToolCall,
        cancellation_token: CancellationToken,
    ) -> Result<JsonValue, String>;

    async fn notify(&self, call_id: String, cell_id: String, text: String) -> Result<(), String>;
}

struct Inner {
    stored_values: Mutex<HashMap<String, JsonValue>>,
    next_cell_id: AtomicU64,
}

pub struct CodeModeService {
    inner: Arc<Inner>,
}

impl CodeModeService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                stored_values: Mutex::new(HashMap::new()),
                next_cell_id: AtomicU64::new(1),
            }),
        }
    }

    pub async fn stored_values(&self) -> HashMap<String, JsonValue> {
        self.inner.stored_values.lock().await.clone()
    }

    pub async fn replace_stored_values(&self, values: HashMap<String, JsonValue>) {
        *self.inner.stored_values.lock().await = values;
    }

    /// Reserves the runtime cell id for a future `execute` request.
    ///
    /// Code mode no longer embeds V8. The id is preserved so hosts can hand the
    /// request to Bun, Node.js, a browser worker, or an EdgeRun compute/runtime
    /// capability while keeping Codex protocol compatibility.
    pub fn allocate_cell_id(&self) -> String {
        self.inner
            .next_cell_id
            .fetch_add(1, Ordering::Relaxed)
            .to_string()
    }

    pub async fn execute(&self, request: ExecuteRequest) -> Result<RuntimeResponse, String> {
        Ok(RuntimeResponse::Result {
            cell_id: request.cell_id,
            content_items: Vec::new(),
            stored_values: request.stored_values,
            error_text: Some(
                "embedded V8 code mode was removed; execute through an external runtime capability such as Bun, Node.js, browser worker, or EdgeRun compute node".to_string(),
            ),
        })
    }

    pub async fn wait(&self, request: WaitRequest) -> Result<WaitOutcome, String> {
        Ok(WaitOutcome::MissingCell(missing_cell_response(request.cell_id)))
    }

    pub fn start_turn_worker(&self, _host: Arc<dyn CodeModeTurnHost>) -> CodeModeTurnWorker {
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        CodeModeTurnWorker {
            shutdown_tx: Some(shutdown_tx),
        }
    }
}

impl Default for CodeModeService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CodeModeTurnWorker {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Drop for CodeModeTurnWorker {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;

    use super::CodeModeService;
    use crate::runtime::ExecuteRequest;
    use crate::runtime::RuntimeResponse;

    fn execute_request(source: &str) -> ExecuteRequest {
        ExecuteRequest {
            cell_id: "1".to_string(),
            tool_call_id: "call_1".to_string(),
            enabled_tools: Vec::new(),
            source: source.to_string(),
            stored_values: HashMap::new(),
            yield_time_ms: Some(1),
            max_output_tokens: None,
        }
    }

    #[edgerun_tokio::test]
    async fn execute_reports_external_runtime_boundary() {
        let service = CodeModeService::new();
        let response = service.execute(execute_request("1 + 1")).await.unwrap();
        let RuntimeResponse::Result { error_text, .. } = response else {
            panic!("expected result response");
        };
        assert_eq!(
            error_text,
            Some(
                "embedded V8 code mode was removed; execute through an external runtime capability such as Bun, Node.js, browser worker, or EdgeRun compute node".to_string(),
            )
        );
    }
}
