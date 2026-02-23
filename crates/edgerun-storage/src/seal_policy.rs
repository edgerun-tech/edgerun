// SPDX-License-Identifier: GPL-2.0-only
use serde::{Deserialize, Serialize};

use crate::StorageError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SealPolicyMode {
    ChainStrict,
    #[default]
    ChainPreferred,
    TimeOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainSealPolicy {
    #[serde(default = "default_max_slot_span")]
    pub max_slot_span: u64,
    #[serde(default = "default_max_epoch_span")]
    pub max_epoch_span: u64,
}

impl Default for ChainSealPolicy {
    fn default() -> Self {
        Self {
            max_slot_span: default_max_slot_span(),
            max_epoch_span: default_max_epoch_span(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeSealPolicy {
    #[serde(default = "default_max_age_ms")]
    pub max_age_ms: u64,
    #[serde(default = "default_idle_ms")]
    pub idle_ms: u64,
}

impl Default for TimeSealPolicy {
    fn default() -> Self {
        Self {
            max_age_ms: default_max_age_ms(),
            idle_ms: default_idle_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StorageSealPolicy {
    #[serde(default)]
    pub mode: SealPolicyMode,
    #[serde(default)]
    pub chain: ChainSealPolicy,
    #[serde(default)]
    pub time: TimeSealPolicy,
}

impl StorageSealPolicy {
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.chain.max_slot_span == 0 {
            return Err(StorageError::InvalidSealPolicy(
                "chain.max_slot_span must be > 0".to_string(),
            ));
        }
        if self.chain.max_epoch_span == 0 {
            return Err(StorageError::InvalidSealPolicy(
                "chain.max_epoch_span must be > 0".to_string(),
            ));
        }
        if self.time.max_age_ms == 0 {
            return Err(StorageError::InvalidSealPolicy(
                "time.max_age_ms must be > 0".to_string(),
            ));
        }
        if self.time.idle_ms == 0 {
            return Err(StorageError::InvalidSealPolicy(
                "time.idle_ms must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainProgress {
    pub slot: u64,
    pub epoch: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveSegmentState {
    pub opened_at_unix_ms: u64,
    pub last_append_unix_ms: u64,
    pub opened_chain: Option<ChainProgress>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SealTrigger {
    ChainSlotSpan,
    ChainEpochSpan,
    TimeMaxAge,
    TimeIdle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealDecision {
    pub should_seal: bool,
    pub trigger: Option<SealTrigger>,
    pub reason: Option<String>,
}

impl SealDecision {
    pub fn no() -> Self {
        Self {
            should_seal: false,
            trigger: None,
            reason: None,
        }
    }

    pub fn yes(trigger: SealTrigger, reason: String) -> Self {
        Self {
            should_seal: true,
            trigger: Some(trigger),
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SealController {
    policy: StorageSealPolicy,
}

impl SealController {
    pub fn new(policy: StorageSealPolicy) -> Result<Self, StorageError> {
        policy.validate()?;
        Ok(Self { policy })
    }

    pub fn policy(&self) -> &StorageSealPolicy {
        &self.policy
    }

    pub fn decide(
        &self,
        now_unix_ms: u64,
        chain_now: Option<ChainProgress>,
        state: ActiveSegmentState,
    ) -> Result<SealDecision, StorageError> {
        match self.policy.mode {
            SealPolicyMode::ChainStrict => self.decide_chain_strict(now_unix_ms, chain_now, state),
            SealPolicyMode::ChainPreferred => {
                if let Some(decision) = self.decide_chain_if_possible(chain_now, state) {
                    Ok(decision)
                } else {
                    Ok(self.decide_time(now_unix_ms, state))
                }
            }
            SealPolicyMode::TimeOnly => Ok(self.decide_time(now_unix_ms, state)),
        }
    }

    fn decide_chain_strict(
        &self,
        _now_unix_ms: u64,
        chain_now: Option<ChainProgress>,
        state: ActiveSegmentState,
    ) -> Result<SealDecision, StorageError> {
        self.decide_chain_if_possible(chain_now, state)
            .ok_or_else(|| {
                StorageError::InvalidSealPolicy(
                    "chain_strict mode requires chain clock and opened_chain state".to_string(),
                )
            })
    }

    fn decide_chain_if_possible(
        &self,
        chain_now: Option<ChainProgress>,
        state: ActiveSegmentState,
    ) -> Option<SealDecision> {
        let now = chain_now?;
        let opened = state.opened_chain?;

        let slot_span = now.slot.saturating_sub(opened.slot);
        if slot_span >= self.policy.chain.max_slot_span {
            return Some(SealDecision::yes(
                SealTrigger::ChainSlotSpan,
                format!(
                    "slot span {} reached max_slot_span {}",
                    slot_span, self.policy.chain.max_slot_span
                ),
            ));
        }

        let epoch_span = now.epoch.saturating_sub(opened.epoch);
        if epoch_span >= self.policy.chain.max_epoch_span {
            return Some(SealDecision::yes(
                SealTrigger::ChainEpochSpan,
                format!(
                    "epoch span {} reached max_epoch_span {}",
                    epoch_span, self.policy.chain.max_epoch_span
                ),
            ));
        }

        Some(SealDecision::no())
    }

    fn decide_time(&self, now_unix_ms: u64, state: ActiveSegmentState) -> SealDecision {
        let age_ms = now_unix_ms.saturating_sub(state.opened_at_unix_ms);
        if age_ms >= self.policy.time.max_age_ms {
            return SealDecision::yes(
                SealTrigger::TimeMaxAge,
                format!(
                    "age {}ms reached max_age_ms {}",
                    age_ms, self.policy.time.max_age_ms
                ),
            );
        }

        let idle_ms = now_unix_ms.saturating_sub(state.last_append_unix_ms);
        if idle_ms >= self.policy.time.idle_ms {
            return SealDecision::yes(
                SealTrigger::TimeIdle,
                format!(
                    "idle {}ms reached idle_ms {}",
                    idle_ms, self.policy.time.idle_ms
                ),
            );
        }

        SealDecision::no()
    }
}

const fn default_max_slot_span() -> u64 {
    128
}

const fn default_max_epoch_span() -> u64 {
    1
}

const fn default_max_age_ms() -> u64 {
    30_000
}

const fn default_idle_ms() -> u64 {
    5_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_policy_defaults_validate() {
        let policy = StorageSealPolicy::default();
        assert!(policy.validate().is_ok());
        assert_eq!(policy.mode, SealPolicyMode::ChainPreferred);
    }

    #[test]
    fn seal_policy_rejects_zero_limits() {
        let mut policy = StorageSealPolicy::default();
        policy.chain.max_slot_span = 0;
        let err = policy.validate().unwrap_err();
        assert!(err.to_string().contains("chain.max_slot_span"));
    }

    #[test]
    fn chain_preferred_uses_chain_when_available() {
        let controller = SealController::new(StorageSealPolicy::default()).unwrap();
        let decision = controller
            .decide(
                1_000_000,
                Some(ChainProgress {
                    slot: 200,
                    epoch: 2,
                }),
                ActiveSegmentState {
                    opened_at_unix_ms: 0,
                    last_append_unix_ms: 999_999,
                    opened_chain: Some(ChainProgress { slot: 10, epoch: 1 }),
                },
            )
            .unwrap();
        assert!(decision.should_seal);
        assert_eq!(decision.trigger, Some(SealTrigger::ChainSlotSpan));
    }

    #[test]
    fn chain_strict_rejects_missing_chain() {
        let policy = StorageSealPolicy {
            mode: SealPolicyMode::ChainStrict,
            ..StorageSealPolicy::default()
        };
        let controller = SealController::new(policy).unwrap();
        let err = controller
            .decide(
                10,
                None,
                ActiveSegmentState {
                    opened_at_unix_ms: 0,
                    last_append_unix_ms: 0,
                    opened_chain: None,
                },
            )
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("chain_strict mode requires chain clock"));
    }
}
