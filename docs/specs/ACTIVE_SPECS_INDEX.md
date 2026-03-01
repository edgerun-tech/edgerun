# Active Specs Index

Generated: 2026-03-01 03:57:44Z

## Summary

- Total specs: 162
- Cataloged: 28
- Active: 14
- Superseded: 10
- Historical: 4
- Uncataloged: 134

## Active

| Spec | Domain | Note |
|---|---|---|
| [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | repo-ops | Current cleanup baseline after removal of agent/executor tool surfaces. |
| [2026-03-01-ci-build-determinism-and-hygiene-wins-v1.md](./2026-03-01-ci-build-determinism-and-hygiene-wins-v1.md) | ci | Batches high-impact CI/build determinism and hygiene fixes. |
| [2026-03-01-integration-lifecycle-class-template-v1.md](./2026-03-01-integration-lifecycle-class-template-v1.md) | integrations | Current integration lifecycle implementation template. |
| [2026-03-01-integration-machine-workflow-contract-v1.md](./2026-03-01-integration-machine-workflow-contract-v1.md) | integrations | Current integration/workflow contract baseline. |
| [2026-03-01-intent-ui-integration-lifecycle-hardening-v1.md](./2026-03-01-intent-ui-integration-lifecycle-hardening-v1.md) | intent-ui | Current hardening direction for lifecycle logic. |
| [2026-03-01-opencode-only-assistant-integration-v1.md](./2026-03-01-opencode-only-assistant-integration-v1.md) | intent-ui | OpenCode is the sole assistant provider and integration path. |
| [2026-03-01-spec-catalog-and-active-index-v1.md](./2026-03-01-spec-catalog-and-active-index-v1.md) | docs | Introduces deterministic status catalog and generated active index. |
| [CI_CD_REQUIRED_CHECKS_WORKFLOW_HYGIENE_AND_DEPLOY_SMOKE_V1.md](./CI_CD_REQUIRED_CHECKS_WORKFLOW_HYGIENE_AND_DEPLOY_SMOKE_V1.md) | ci | Required checks policy remains enforced. |
| [CLOUDFLARE_FRONTEND_TARGETS_V1.md](./CLOUDFLARE_FRONTEND_TARGETS_V1.md) | frontend-delivery | Frontend deploy target pinning policy remains active. |
| [DOCS_CONTEXT_CONSOLIDATION_V1.md](./DOCS_CONTEXT_CONSOLIDATION_V1.md) | docs | Compact context docs are current operator baseline. |
| [FRONTEND_PRODUCTION_READINESS_GATES_V1.md](./FRONTEND_PRODUCTION_READINESS_GATES_V1.md) | frontend-quality | Production gates for frontend are still active. |
| [FRONTEND_ROUTED_WS_AND_A11Y_POLISH_V1.md](./FRONTEND_ROUTED_WS_AND_A11Y_POLISH_V1.md) | frontend-quality | Routed WS and accessibility contract remains active. |
| [NODE_MANAGER_DOCKER_COMPOSE_BOOTSTRAP_V1.md](./NODE_MANAGER_DOCKER_COMPOSE_BOOTSTRAP_V1.md) | node-manager | Compose bootstrap remains active operational path. |
| [UNIFIED_WORKFLOW_ECOSYSTEM_V1.md](./UNIFIED_WORKFLOW_ECOSYSTEM_V1.md) | ci | Unified check/build/test/verify workflow remains canonical. |

## Superseded

| Spec | Replaced By | Note |
|---|---|---|
| [2026-02-28-agentic-runtime-minimal-architecture-v1.md](./2026-02-28-agentic-runtime-minimal-architecture-v1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | Runtime entrypoints referenced removed tooling wrappers. |
| [2026-02-28-agent-storage-native-proposal-flow-v1.md](./2026-02-28-agent-storage-native-proposal-flow-v1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | Storage proposal wrapper flow removed with tooling surface cleanup. |
| [2026-03-01-agent-virtualized-context-diff-events-and-test-executors-v1.md](./2026-03-01-agent-virtualized-context-diff-events-and-test-executors-v1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | Defined flows via removed agent script surface. |
| [2026-03-01-ai-native-storage-agent-workflow-v1.md](./2026-03-01-ai-native-storage-agent-workflow-v1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | Assumed edgertool-based autosubmit path. |
| [2026-03-01-codex-assistant-direct-local-backend-v1.md](./2026-03-01-codex-assistant-direct-local-backend-v1.md) | [2026-03-01-opencode-only-assistant-integration-v1.md](./2026-03-01-opencode-only-assistant-integration-v1.md) | Codex-specific backend wiring replaced by OpenCode-only path. |
| [2026-03-01-codex-session-persistence-and-resume-v1.md](./2026-03-01-codex-session-persistence-and-resume-v1.md) | [2026-03-01-opencode-only-assistant-integration-v1.md](./2026-03-01-opencode-only-assistant-integration-v1.md) | Session persistence now follows OpenCode assistant execution. |
| [2026-03-01-intent-ui-codex-cli-availability-without-profile-v1.md](./2026-03-01-intent-ui-codex-cli-availability-without-profile-v1.md) | [2026-03-01-opencode-only-assistant-integration-v1.md](./2026-03-01-opencode-only-assistant-integration-v1.md) | Codex CLI availability policy replaced by OpenCode CLI policy. |
| [AGENT_DIFF_EVENT_FIRST_ACCEPTANCE_V1.md](./AGENT_DIFF_EVENT_FIRST_ACCEPTANCE_V1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | Acceptance flow depended on removed scripts. |
| [AGENT_WORKFLOW_SCRIPT_RESILIENCE_V1.md](./AGENT_WORKFLOW_SCRIPT_RESILIENCE_V1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | Referenced scripts were removed. |
| [GO_TOOLING_EVENT_WORKFLOW_V1.md](./GO_TOOLING_EVENT_WORKFLOW_V1.md) | [2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md](./2026-03-01-agent-tooling-surface-removal-and-repo-coherence-v1.md) | edgertool-based workflow removed. |

## Historical

| Spec | Domain | Note |
|---|---|---|
| [2026-02-24-cloud-os-eslint-config.md](./2026-02-24-cloud-os-eslint-config.md) | cloud-os | Cloud-os scope removed from current repo surface. |
| [2026-03-01-nats-jetstream-central-eventbus-and-container-agents-v1.md](./2026-03-01-nats-jetstream-central-eventbus-and-container-agents-v1.md) | eventbus | Aspirational architecture; current implementation diverged. |
| [CLOUD_OS_CODEX_CLI_PIPELINE_V1.md](./CLOUD_OS_CODEX_CLI_PIPELINE_V1.md) | cloud-os | Cloud-os pipeline is not in current active tree. |
| [CLOUD_OS_DEV_HMR_AND_SOLID_RUNTIME_STABILITY_V1.md](./CLOUD_OS_DEV_HMR_AND_SOLID_RUNTIME_STABILITY_V1.md) | cloud-os | Cloud-os dev loop no longer active in current repository layout. |

## Uncataloged

These specs exist but do not yet have status metadata in `spec-status.tsv`.

- [2026-02-24-dashboard-chain-metrics-deterministic-cypress-v1.md](./2026-02-24-dashboard-chain-metrics-deterministic-cypress-v1.md)
- [2026-02-24-frontend-bun-only-build-pipeline-v1.md](./2026-02-24-frontend-bun-only-build-pipeline-v1.md)
- [2026-02-24-frontend-email-collection-cloudflare-kv-v1.md](./2026-02-24-frontend-email-collection-cloudflare-kv-v1.md)
- [2026-02-24-mock-surface-elimination-v1.md](./2026-02-24-mock-surface-elimination-v1.md)
- [2026-02-24-remove-remaining-runtime-mocks-v1.md](./2026-02-24-remove-remaining-runtime-mocks-v1.md)
- [2026-02-24-scheduler-ingestion-and-dialing-enforcement.md](./2026-02-24-scheduler-ingestion-and-dialing-enforcement.md)
- [2026-02-25-frontend-landing-mystic-hero-stats-v1.md](./2026-02-25-frontend-landing-mystic-hero-stats-v1.md)
- [2026-02-26-ci-ground-up-rebuild-v1.md](./2026-02-26-ci-ground-up-rebuild-v1.md)
- [2026-02-26-edgerun-containerd-shim-and-eventstream-snapshotter-v1.md](./2026-02-26-edgerun-containerd-shim-and-eventstream-snapshotter-v1.md)
- [2026-02-26-edgerun-snapshotter-decommission-v1.md](./2026-02-26-edgerun-snapshotter-decommission-v1.md)
- [2026-02-26-rbi-state-stream-protocol-v1.md](./2026-02-26-rbi-state-stream-protocol-v1.md)
- [2026-02-26-remove-serde-json-completely-v1.md](./2026-02-26-remove-serde-json-completely-v1.md)
- [2026-02-26-remove-vanity-generator-v1.md](./2026-02-26-remove-vanity-generator-v1.md)
- [2026-02-26-route-candidates-control-plane-v1.md](./2026-02-26-route-candidates-control-plane-v1.md)
- [2026-02-26-route-candidate-scoring-v1.md](./2026-02-26-route-candidate-scoring-v1.md)
- [2026-02-26-route-stun-reflexive-candidate-v1.md](./2026-02-26-route-stun-reflexive-candidate-v1.md)
- [2026-02-26-scheduler-worker-remove-solana-deps-v1.md](./2026-02-26-scheduler-worker-remove-solana-deps-v1.md)
- [2026-02-26-serde-json-removal-eventbus-rpc-v1.md](./2026-02-26-serde-json-removal-eventbus-rpc-v1.md)
- [2026-02-26-worker-policy-refresh-404-fallback-v1.md](./2026-02-26-worker-policy-refresh-404-fallback-v1.md)
- [2026-02-27-ci-subminute-cache-lane-v1.md](./2026-02-27-ci-subminute-cache-lane-v1.md)
- [2026-02-27-intent-ui-obsolete-artifacts-cleanup-v1.md](./2026-02-27-intent-ui-obsolete-artifacts-cleanup-v1.md)
- [2026-02-27-intent-ui-pipeline-unification-v1.md](./2026-02-27-intent-ui-pipeline-unification-v1.md)
- [2026-02-27-intent-ui-route-integration-v1.md](./2026-02-27-intent-ui-route-integration-v1.md)
- [2026-02-27-local-subminute-rust-build-v1.md](./2026-02-27-local-subminute-rust-build-v1.md)
- [2026-02-28-device-capability-bluetooth-nfc-v1.md](./2026-02-28-device-capability-bluetooth-nfc-v1.md)
- [2026-02-28-device-capability-effective-availability-benchmarks-v1.md](./2026-02-28-device-capability-effective-availability-benchmarks-v1.md)
- [2026-02-28-device-capability-host-android-windows-v1.md](./2026-02-28-device-capability-host-android-windows-v1.md)
- [2026-02-28-device-capability-host-linux-v1.md](./2026-02-28-device-capability-host-linux-v1.md)
- [2026-02-28-device-capability-probing-core-v1.md](./2026-02-28-device-capability-probing-core-v1.md)
- [2026-02-28-device-capability-protobuf-contracts-v1.md](./2026-02-28-device-capability-protobuf-contracts-v1.md)
- [2026-02-28-device-capability-scheduler-eligibility-v1.md](./2026-02-28-device-capability-scheduler-eligibility-v1.md)
- [2026-02-28-efi-network-boot-policy-v1.md](./2026-02-28-efi-network-boot-policy-v1.md)
- [2026-02-28-integrations-mcp-runtime-and-github-direct-v1.md](./2026-02-28-integrations-mcp-runtime-and-github-direct-v1.md)
- [2026-02-28-intent-pipeline-contracts-v1.md](./2026-02-28-intent-pipeline-contracts-v1.md)
- [2026-02-28-intent-ui-device-connect-dialog-v1.md](./2026-02-28-intent-ui-device-connect-dialog-v1.md)
- [2026-02-28-intent-ui-device-pairing-code-issue-v1.md](./2026-02-28-intent-ui-device-pairing-code-issue-v1.md)
- [2026-02-28-intent-ui-file-manager-node-targeting-v1.md](./2026-02-28-intent-ui-file-manager-node-targeting-v1.md)
- [2026-02-28-intent-ui-frontend-consolidation-v1.md](./2026-02-28-intent-ui-frontend-consolidation-v1.md)
- [2026-02-28-intent-ui-integration-secrets-encrypted-profile-v1.md](./2026-02-28-intent-ui-integration-secrets-encrypted-profile-v1.md)
- [2026-02-28-intent-ui-integrations-event-intent-stepper-v1.md](./2026-02-28-intent-ui-integrations-event-intent-stepper-v1.md)
- [2026-02-28-intent-ui-linux-first-device-connection-v1.md](./2026-02-28-intent-ui-linux-first-device-connection-v1.md)
- [2026-02-28-intent-ui-local-bridge-strict-mode-v1.md](./2026-02-28-intent-ui-local-bridge-strict-mode-v1.md)
- [2026-02-28-intent-ui-remove-profile-onboarding-v1.md](./2026-02-28-intent-ui-remove-profile-onboarding-v1.md)
- [2026-02-28-intent-ui-tailscale-api-routing-v1.md](./2026-02-28-intent-ui-tailscale-api-routing-v1.md)
- [2026-02-28-intent-ui-tailscale-app-connector-setup-v1.md](./2026-02-28-intent-ui-tailscale-app-connector-setup-v1.md)
- [2026-02-28-intent-ui-tailscale-integration-v1.md](./2026-02-28-intent-ui-tailscale-integration-v1.md)
- [2026-02-28-intent-ui-tailscale-stepper-ux-v1.md](./2026-02-28-intent-ui-tailscale-stepper-ux-v1.md)
- [2026-02-28-libp2p-unification-plan-v1.md](./2026-02-28-libp2p-unification-plan-v1.md)
- [2026-02-28-linux-node-manager-installer-service-v1.md](./2026-02-28-linux-node-manager-installer-service-v1.md)
- [2026-02-28-local-first-solana-removal-phase1-v1.md](./2026-02-28-local-first-solana-removal-phase1-v1.md)

- ... and 84 more uncataloged specs
