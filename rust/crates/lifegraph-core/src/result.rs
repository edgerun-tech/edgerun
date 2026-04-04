use crate::value::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Accept,
    Reject,
    Defer,
    Duplicate,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Accept => "ACCEPT",
            Verdict::Reject => "REJECT",
            Verdict::Defer => "DEFER",
            Verdict::Duplicate => "DUPLICATE",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReasonCode {
    StructuralInvalid,
    CryptoInvalid,
    VersionUnsupported,
    TargetMismatch,
    ReplayDetected,
    AuthorityDenied,
    PolicyDenied,
    MissingDependency,
    ForkConflict,
    CanonicalizationFail,
    ObjectIdMismatch,
    SnapshotBaseConflict,
    TimeInvalid,
    RevocationActive,
    AssuranceInsufficient,
    ControlInvariantFailed,
    RepresentationInvalid,
}

impl ReasonCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasonCode::StructuralInvalid => "STRUCTURAL_INVALID",
            ReasonCode::CryptoInvalid => "CRYPTO_INVALID",
            ReasonCode::VersionUnsupported => "VERSION_UNSUPPORTED",
            ReasonCode::TargetMismatch => "TARGET_MISMATCH",
            ReasonCode::ReplayDetected => "REPLAY_DETECTED",
            ReasonCode::AuthorityDenied => "AUTHORITY_DENIED",
            ReasonCode::PolicyDenied => "POLICY_DENIED",
            ReasonCode::MissingDependency => "MISSING_DEPENDENCY",
            ReasonCode::ForkConflict => "FORK_CONFLICT",
            ReasonCode::CanonicalizationFail => "CANONICALIZATION_FAILED",
            ReasonCode::ObjectIdMismatch => "OBJECT_ID_MISMATCH",
            ReasonCode::SnapshotBaseConflict => "SNAPSHOT_BASE_CONFLICT",
            ReasonCode::TimeInvalid => "TIME_INVALID",
            ReasonCode::RevocationActive => "REVOCATION_ACTIVE",
            ReasonCode::AssuranceInsufficient => "ASSURANCE_INSUFFICIENT",
            ReasonCode::ControlInvariantFailed => "CONTROL_INVARIANT_FAILED",
            ReasonCode::RepresentationInvalid => "REPRESENTATION_INVALID",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub verdict: Verdict,
    pub reason_code: Option<ReasonCode>,
    pub derived: Value,
    pub post_state: Value,
}

pub fn empty_map() -> Value {
    Value::Map(Default::default())
}

pub fn accept(derived: Value, post_state: Value) -> ValidationResult {
    ValidationResult {
        verdict: Verdict::Accept,
        reason_code: None,
        derived,
        post_state,
    }
}

pub fn reject(code: ReasonCode, derived: Value, post_state: Value) -> ValidationResult {
    ValidationResult {
        verdict: Verdict::Reject,
        reason_code: Some(code),
        derived,
        post_state,
    }
}

pub fn defer(code: ReasonCode, derived: Value) -> ValidationResult {
    ValidationResult {
        verdict: Verdict::Defer,
        reason_code: Some(code),
        derived,
        post_state: empty_map(),
    }
}

pub fn duplicate(code: ReasonCode, derived: Value) -> ValidationResult {
    ValidationResult {
        verdict: Verdict::Duplicate,
        reason_code: Some(code),
        derived,
        post_state: empty_map(),
    }
}
