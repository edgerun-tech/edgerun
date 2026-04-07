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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Verdict::as_str ----

    #[test]
    fn verdict_accept_str() {
        assert_eq!(Verdict::Accept.as_str(), "ACCEPT");
    }

    #[test]
    fn verdict_reject_str() {
        assert_eq!(Verdict::Reject.as_str(), "REJECT");
    }

    #[test]
    fn verdict_defer_str() {
        assert_eq!(Verdict::Defer.as_str(), "DEFER");
    }

    #[test]
    fn verdict_duplicate_str() {
        assert_eq!(Verdict::Duplicate.as_str(), "DUPLICATE");
    }

    // ---- ReasonCode::as_str (all 16 variants) ----

    #[test]
    fn reason_code_structural_invalid() {
        assert_eq!(ReasonCode::StructuralInvalid.as_str(), "STRUCTURAL_INVALID");
    }

    #[test]
    fn reason_code_crypto_invalid() {
        assert_eq!(ReasonCode::CryptoInvalid.as_str(), "CRYPTO_INVALID");
    }

    #[test]
    fn reason_code_version_unsupported() {
        assert_eq!(ReasonCode::VersionUnsupported.as_str(), "VERSION_UNSUPPORTED");
    }

    #[test]
    fn reason_code_target_mismatch() {
        assert_eq!(ReasonCode::TargetMismatch.as_str(), "TARGET_MISMATCH");
    }

    #[test]
    fn reason_code_replay_detected() {
        assert_eq!(ReasonCode::ReplayDetected.as_str(), "REPLAY_DETECTED");
    }

    #[test]
    fn reason_code_authority_denied() {
        assert_eq!(ReasonCode::AuthorityDenied.as_str(), "AUTHORITY_DENIED");
    }

    #[test]
    fn reason_code_policy_denied() {
        assert_eq!(ReasonCode::PolicyDenied.as_str(), "POLICY_DENIED");
    }

    #[test]
    fn reason_code_missing_dependency() {
        assert_eq!(ReasonCode::MissingDependency.as_str(), "MISSING_DEPENDENCY");
    }

    #[test]
    fn reason_code_fork_conflict() {
        assert_eq!(ReasonCode::ForkConflict.as_str(), "FORK_CONFLICT");
    }

    #[test]
    fn reason_code_canonicalization_failed() {
        assert_eq!(ReasonCode::CanonicalizationFail.as_str(), "CANONICALIZATION_FAILED");
    }

    #[test]
    fn reason_code_object_id_mismatch() {
        assert_eq!(ReasonCode::ObjectIdMismatch.as_str(), "OBJECT_ID_MISMATCH");
    }

    #[test]
    fn reason_code_snapshot_base_conflict() {
        assert_eq!(ReasonCode::SnapshotBaseConflict.as_str(), "SNAPSHOT_BASE_CONFLICT");
    }

    #[test]
    fn reason_code_time_invalid() {
        assert_eq!(ReasonCode::TimeInvalid.as_str(), "TIME_INVALID");
    }

    #[test]
    fn reason_code_revocation_active() {
        assert_eq!(ReasonCode::RevocationActive.as_str(), "REVOCATION_ACTIVE");
    }

    #[test]
    fn reason_code_assurance_insufficient() {
        assert_eq!(ReasonCode::AssuranceInsufficient.as_str(), "ASSURANCE_INSUFFICIENT");
    }

    #[test]
    fn reason_code_control_invariant_failed() {
        assert_eq!(ReasonCode::ControlInvariantFailed.as_str(), "CONTROL_INVARIANT_FAILED");
    }

    #[test]
    fn reason_code_representation_invalid() {
        assert_eq!(ReasonCode::RepresentationInvalid.as_str(), "REPRESENTATION_INVALID");
    }

    // ---- Builder functions ----

    #[test]
    fn accept_result_has_no_reason_code() {
        let r = accept(Value::Null, Value::Null);
        assert_eq!(r.verdict, Verdict::Accept);
        assert!(r.reason_code.is_none());
    }

    #[test]
    fn accept_result_carries_derived_and_post_state() {
        let d = Value::String("hello".into());
        let p = Value::Int(42);
        let r = accept(d.clone(), p.clone());
        assert_eq!(r.derived, d);
        assert_eq!(r.post_state, p);
    }

    #[test]
    fn reject_result_has_reason_code() {
        let r = reject(ReasonCode::TargetMismatch, Value::Null, Value::Null);
        assert_eq!(r.verdict, Verdict::Reject);
        assert_eq!(r.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn reject_carries_derived_and_post_state() {
        let d = Value::Int(1);
        let p = Value::Int(2);
        let r = reject(ReasonCode::PolicyDenied, d.clone(), p.clone());
        assert_eq!(r.derived, d);
        assert_eq!(r.post_state, p);
    }

    #[test]
    fn defer_result_has_reason_code() {
        let r = defer(ReasonCode::MissingDependency, Value::String("x".into()));
        assert_eq!(r.verdict, Verdict::Defer);
        assert_eq!(r.reason_code, Some(ReasonCode::MissingDependency));
        assert_eq!(r.derived, Value::String("x".into()));
    }

    #[test]
    fn defer_has_empty_post_state() {
        let r = defer(ReasonCode::MissingDependency, Value::Null);
        assert_eq!(r.post_state, empty_map());
    }

    #[test]
    fn duplicate_result_has_reason_code() {
        let r = duplicate(ReasonCode::ReplayDetected, Value::Null);
        assert_eq!(r.verdict, Verdict::Duplicate);
        assert_eq!(r.reason_code, Some(ReasonCode::ReplayDetected));
    }

    #[test]
    fn duplicate_has_empty_post_state() {
        let r = duplicate(ReasonCode::ReplayDetected, Value::Null);
        assert_eq!(r.post_state, empty_map());
    }

    // ---- empty_map ----

    #[test]
    fn empty_map_is_empty_btree_map() {
        let m = empty_map();
        assert_eq!(m, Value::Map(Default::default()));
    }

    // ---- PartialEq / Eq / Clone / Debug for Verdict ----

    #[test]
    fn verdict_equality() {
        assert_eq!(Verdict::Accept, Verdict::Accept);
        assert_ne!(Verdict::Accept, Verdict::Reject);
    }

    #[test]
    fn verdict_clone_and_debug() {
        let v = Verdict::Defer;
        let v2 = v.clone();
        assert_eq!(v, v2);
        let debug_str = format!("{:?}", v);
        assert!(debug_str.contains("Defer"));
    }

    // ---- PartialEq / Eq / Clone / Debug for ReasonCode ----

    #[test]
    fn reason_code_equality() {
        assert_eq!(ReasonCode::CryptoInvalid, ReasonCode::CryptoInvalid);
        assert_ne!(ReasonCode::CryptoInvalid, ReasonCode::StructuralInvalid);
    }

    #[test]
    fn reason_code_clone_and_debug() {
        let r = ReasonCode::PolicyDenied;
        let r2 = r.clone();
        assert_eq!(r, r2);
        let debug_str = format!("{:?}", r);
        assert!(debug_str.contains("PolicyDenied"));
    }

    // ---- ValidationResult Clone / Debug ----

    #[test]
    fn validation_result_clone_debug() {
        let vr = ValidationResult {
            verdict: Verdict::Accept,
            reason_code: None,
            derived: Value::Bool(true),
            post_state: empty_map(),
        };
        let vr2 = vr.clone();
        assert_eq!(vr.verdict, vr2.verdict);
        let debug_str = format!("{:?}", vr);
        assert!(debug_str.contains("Accept"));
    }
}
