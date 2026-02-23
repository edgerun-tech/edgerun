// SPDX-License-Identifier: Apache-2.0
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsJobSpec {
    pub job_id: String,
    pub run_id: String,
    pub attempt: u32,
    pub label_selector: Vec<String>,
    pub bundle_payload: Vec<u8>,
    pub expected_runtime_id: Option<[u8; 32]>,
    pub expected_abi_version: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsJobLease {
    pub lease_id: String,
    pub spec: ActionsJobSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionsJobState {
    Queued,
    InProgress,
    Succeeded,
    Failed,
    Cancelled,
    InfrastructureError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsJobResult {
    pub state: ActionsJobState,
    pub output_hash: Option<[u8; 32]>,
    pub output_len: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    ContractViolation(&'static str),
    ControlPlane(String),
    Executor(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::ContractViolation(msg) => write!(f, "contract violation: {msg}"),
            BackendError::ControlPlane(msg) => write!(f, "control plane error: {msg}"),
            BackendError::Executor(msg) => write!(f, "executor error: {msg}"),
        }
    }
}

impl std::error::Error for BackendError {}

pub trait ControlPlane {
    fn fetch_next_job(&mut self) -> Result<Option<ActionsJobLease>, BackendError>;
    fn append_log_chunk(&mut self, lease: &ActionsJobLease, text: &str)
        -> Result<(), BackendError>;
    fn report_state(
        &mut self,
        lease: &ActionsJobLease,
        state: ActionsJobState,
        detail: &str,
    ) -> Result<(), BackendError>;
    fn report_result(
        &mut self,
        lease: &ActionsJobLease,
        result: &ActionsJobResult,
    ) -> Result<(), BackendError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionDisposition {
    Succeeded {
        output_hash: [u8; 32],
        output_len: usize,
    },
    Failed {
        detail: String,
    },
    Cancelled {
        detail: String,
    },
    InfrastructureError {
        detail: String,
    },
    Unsupported {
        detail: String,
    },
}

pub trait JobExecutor {
    fn execute(&mut self, lease: &ActionsJobLease) -> Result<ExecutionDisposition, BackendError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobTickOutcome {
    Idle,
    Executed,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopJobExecutor;

impl JobExecutor for NoopJobExecutor {
    fn execute(&mut self, lease: &ActionsJobLease) -> Result<ExecutionDisposition, BackendError> {
        let detail = format!(
            "no-op executor: job_id={} run_id={} attempt={} was acknowledged but not executed",
            lease.spec.job_id, lease.spec.run_id, lease.spec.attempt
        );
        Ok(ExecutionDisposition::Unsupported { detail })
    }
}

pub struct RunnerBackend<C, E>
where
    C: ControlPlane,
    E: JobExecutor,
{
    control_plane: C,
    executor: E,
}

impl<C, E> RunnerBackend<C, E>
where
    C: ControlPlane,
    E: JobExecutor,
{
    pub fn new(control_plane: C, executor: E) -> Self {
        Self {
            control_plane,
            executor,
        }
    }

    pub fn run_tick(&mut self) -> Result<JobTickOutcome, BackendError> {
        let Some(lease) = self.control_plane.fetch_next_job()? else {
            return Ok(JobTickOutcome::Idle);
        };

        let _ = self.control_plane.report_state(
            &lease,
            ActionsJobState::InProgress,
            "job lease acquired by edgerun-runtime backend",
        );

        // Determinism invariant:
        // - a lease is executed at most once in this tick
        // - local/reporting failures MUST NOT trigger local re-execution
        // - retries must be scheduler-issued as a new job/attempt
        let disposition = match self.executor.execute(&lease) {
            Ok(value) => value,
            Err(err) => ExecutionDisposition::InfrastructureError {
                detail: format!("executor_failed_without_retry: {err}"),
            },
        };
        let result = map_disposition(disposition.clone());

        let mut reporting_errors = Vec::new();

        if let Err(err) = self.control_plane.append_log_chunk(
            &lease,
            &format!(
                "[edgerun-runtime] terminal_state={:?} detail={}",
                result.state, result.detail
            ),
        ) {
            reporting_errors.push(format!("append_log_chunk: {err}"));
        }
        if let Err(err) = self
            .control_plane
            .report_state(&lease, result.state, &result.detail)
        {
            reporting_errors.push(format!("report_state: {err}"));
        }
        if let Err(err) = self.control_plane.report_result(&lease, &result) {
            reporting_errors.push(format!("report_result: {err}"));
        }

        if !reporting_errors.is_empty() {
            return Err(BackendError::ControlPlane(format!(
                "post_exec_reporting_failed_without_retry lease_id={} job_id={} run_id={} errors={}",
                lease.lease_id,
                lease.spec.job_id,
                lease.spec.run_id,
                reporting_errors.join("; ")
            )));
        }

        Ok(JobTickOutcome::Executed)
    }

    pub fn into_parts(self) -> (C, E) {
        (self.control_plane, self.executor)
    }
}

fn map_disposition(disposition: ExecutionDisposition) -> ActionsJobResult {
    match disposition {
        ExecutionDisposition::Succeeded {
            output_hash,
            output_len,
        } => ActionsJobResult {
            state: ActionsJobState::Succeeded,
            output_hash: Some(output_hash),
            output_len: Some(output_len),
            detail: "execution completed".to_string(),
        },
        ExecutionDisposition::Failed { detail } => ActionsJobResult {
            state: ActionsJobState::Failed,
            output_hash: None,
            output_len: None,
            detail,
        },
        ExecutionDisposition::Cancelled { detail } => ActionsJobResult {
            state: ActionsJobState::Cancelled,
            output_hash: None,
            output_len: None,
            detail,
        },
        ExecutionDisposition::InfrastructureError { detail } => ActionsJobResult {
            state: ActionsJobState::InfrastructureError,
            output_hash: None,
            output_len: None,
            detail,
        },
        ExecutionDisposition::Unsupported { detail } => ActionsJobResult {
            state: ActionsJobState::InfrastructureError,
            output_hash: None,
            output_len: None,
            detail,
        },
    }
}

// TODO(gha-runner): Implement a production executor that verifies
// expected_runtime_id/expected_abi_version and delegates into
// execute_bundle_payload_bytes_for_runtime_and_abi_strict.
// TODO(gha-runner): Replace coarse log/status calls with a durable protocol
// including sequence numbers and at-least-once retry semantics.

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeControlPlane {
        next_job: Option<ActionsJobLease>,
        states: Vec<ActionsJobState>,
        logs: Vec<String>,
        results: Vec<ActionsJobResult>,
    }

    impl FakeControlPlane {
        fn with_one_job() -> Self {
            Self {
                next_job: Some(ActionsJobLease {
                    lease_id: "lease-1".to_string(),
                    spec: ActionsJobSpec {
                        job_id: "job-1".to_string(),
                        run_id: "run-1".to_string(),
                        attempt: 1,
                        label_selector: vec!["self-hosted".to_string()],
                        bundle_payload: vec![0, 1, 2],
                        expected_runtime_id: Some([9_u8; 32]),
                        expected_abi_version: Some(1),
                    },
                }),
                states: Vec::new(),
                logs: Vec::new(),
                results: Vec::new(),
            }
        }
    }

    impl ControlPlane for FakeControlPlane {
        fn fetch_next_job(&mut self) -> Result<Option<ActionsJobLease>, BackendError> {
            Ok(self.next_job.take())
        }

        fn append_log_chunk(
            &mut self,
            _lease: &ActionsJobLease,
            text: &str,
        ) -> Result<(), BackendError> {
            self.logs.push(text.to_string());
            Ok(())
        }

        fn report_state(
            &mut self,
            _lease: &ActionsJobLease,
            state: ActionsJobState,
            _detail: &str,
        ) -> Result<(), BackendError> {
            self.states.push(state);
            Ok(())
        }

        fn report_result(
            &mut self,
            _lease: &ActionsJobLease,
            result: &ActionsJobResult,
        ) -> Result<(), BackendError> {
            self.results.push(result.clone());
            Ok(())
        }
    }

    #[test]
    fn noop_runner_reports_terminal_state() {
        let control_plane = FakeControlPlane::with_one_job();
        let executor = NoopJobExecutor;
        let mut backend = RunnerBackend::new(control_plane, executor);

        let outcome = backend.run_tick().expect("tick");
        assert_eq!(outcome, JobTickOutcome::Executed);

        let (control_plane, _) = backend.into_parts();
        assert_eq!(
            control_plane.states,
            vec![
                ActionsJobState::InProgress,
                ActionsJobState::InfrastructureError
            ]
        );
        assert_eq!(control_plane.results.len(), 1);
        assert_eq!(
            control_plane.results[0].state,
            ActionsJobState::InfrastructureError
        );
        assert!(control_plane.logs[0].contains("terminal_state=InfrastructureError"));
    }

    #[test]
    fn noop_runner_is_idle_when_queue_is_empty() {
        let control_plane = FakeControlPlane {
            next_job: None,
            states: Vec::new(),
            logs: Vec::new(),
            results: Vec::new(),
        };
        let executor = NoopJobExecutor;
        let mut backend = RunnerBackend::new(control_plane, executor);
        let outcome = backend.run_tick().expect("tick");
        assert_eq!(outcome, JobTickOutcome::Idle);
    }
}
