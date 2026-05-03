export type AgentStatus = 'idle' | 'working' | 'blocked' | 'error'

export interface AgentOperatingContract {
  version: string
  principles: AgentPrinciples
  invariants: ProtocolInvariants
  verification: VerificationRequirements
}

export interface AgentPrinciples {
  primaryGoal: string
  repositoryBaseline: string[]
  codingPractices: string[]
  rustFootguns: string[]
  typescriptPractices: string[]
}

export interface ProtocolInvariants {
  commandDeliveryNotAuthority: boolean
  authoritativeStateOnlyThroughEventLog: boolean
  objectsImmutable: boolean
  capabilitiesExplicitScopedConstrained: boolean
  delegationMustAttenuate: boolean
  appsNoAmbientAccess: boolean
  stateChangesThroughCommandsOrApprovals: boolean
  userPresenceRequiredForSensitiveActions: boolean
}

export interface VerificationRequirements {
  alwaysVerifyCompletion: boolean
  runTargetedChecks: boolean
  useRealTests: boolean
  noFakeCompletion: boolean
}

export const DEFAULT_CONTRACT: AgentOperatingContract = {
  version: '1.0.0',
  principles: {
    primaryGoal: 'Complete requested tasks with minimal user burden',
    repositoryBaseline: [
      'Start from protocol, not crate names',
      'Read AGENTS.md before making changes',
      'Generated protobuf/domain types are canonical',
      'Existing EdgeRun crates preferred over external dependencies',
    ],
    codingPractices: [
      'Prefer enums over stringly typed state',
      'Prefer typed IDs/newtypes over raw strings',
      'Prefer explicit state machines over scattered booleans',
      'Prefer exhaustive match/switch handling',
      'Unknown values must map to safe states',
      'Use defensive parsing and validation',
      'Validate input early and return typed errors',
      'Never silently ignore security-critical unknowns',
    ],
    rustFootguns: [
      'Do not hold locks across await points',
      'Do not block async/runtime threads with long sync work',
      'Do not clone large buffers unnecessarily',
      'Avoid global mutable state unless clear sync model',
      'Avoid unsafe unless strictly necessary',
      'Do not use lossy numeric casts for protocol values',
      'Validate lengths before slicing/indexing',
      'Treat external bytes/JSON/packets as hostile',
    ],
    typescriptPractices: [
      'Components render only',
      'Pages compose only',
      'Hooks access platform state/actions',
      'Stores own state',
      'Registries normalize lookup/discovery',
      'Trackers own auth/permission/approval lifecycle',
    ],
  },
  invariants: {
    commandDeliveryNotAuthority: true,
    authoritativeStateOnlyThroughEventLog: true,
    objectsImmutable: true,
    capabilitiesExplicitScopedConstrained: true,
    delegationMustAttenuate: true,
    appsNoAmbientAccess: true,
    stateChangesThroughCommandsOrApprovals: true,
    userPresenceRequiredForSensitiveActions: true,
  },
  verification: {
    alwaysVerifyCompletion: true,
    runTargetedChecks: true,
    useRealTests: true,
    noFakeCompletion: true,
  },
}

export function validateContract(contract: Partial<AgentOperatingContract>): boolean {
  if (!contract.version || !contract.principles || !contract.invariants) {
    return false
  }
  return Object.values(contract.invariants).every(v => v === true)
}
