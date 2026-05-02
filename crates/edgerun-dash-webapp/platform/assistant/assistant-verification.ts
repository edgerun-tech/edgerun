import { atom, computed } from 'nanostores'

export type VerificationStatus = 'passed' | 'failed' | 'not_run' | 'running'

export interface VerificationResult {
  status: VerificationStatus
  checks: VerificationCheck[]
  timestamp: number
  duration?: number
}

export interface VerificationCheck {
  name: string
  passed: boolean
  error?: string
  warning?: string
}

export interface VerificationConfig {
  runRustTests: boolean
  runTypecheck: boolean
  runLint: boolean
  runBuild: boolean
  runUnitTests: boolean
}

const DEFAULT_CONFIG: VerificationConfig = {
  runRustTests: true,
  runTypecheck: true,
  runLint: false,
  runBuild: true,
  runUnitTests: true,
}

export const verificationStore = atom<{
  status: VerificationStatus
  lastResult: VerificationResult | null
  config: VerificationConfig
  isRunning: boolean
}>({
  status: 'not_run',
  lastResult: null,
  config: DEFAULT_CONFIG,
  isRunning: false,
})

export const verificationStatus = computed(verificationStore, (state) => state.status)
export const verificationResult = computed(verificationStore, (state) => state.lastResult)
export const isVerifying = computed(verificationStore, (state) => state.isRunning)

export function setVerificationConfig(config: Partial<VerificationConfig>): void {
  const state = verificationStore.get()
  verificationStore.set({
    ...state,
    config: { ...state.config, ...config },
  })
}

export function startVerification(): void {
  verificationStore.set((state) => ({
    ...state,
    status: 'running',
    isRunning: true,
  }))
}

export function completeVerification(result: VerificationResult): void {
  const allPassed = result.checks.every((c) => c.passed)
  verificationStore.set({
    status: allPassed ? 'passed' : 'failed',
    lastResult: result,
    isRunning: false,
  })
}

export function failVerification(error: string): void {
  verificationStore.set({
    status: 'failed',
    lastResult: {
      status: 'failed',
      checks: [{ name: 'verification', passed: false, error }],
      timestamp: Date.now(),
    },
    isRunning: false,
  })
}

export function resetVerification(): void {
  verificationStore.set({
    status: 'not_run',
    lastResult: null,
    isRunning: false,
  })
}

export function getVerificationSummary(): string {
  const state = verificationStore.get()
  if (state.status === 'not_run') return 'Not run'
  if (state.status === 'running') return 'Running...'
  if (!state.lastResult) return 'No result'

  const passed = state.lastResult.checks.filter((c) => c.passed).length
  const total = state.lastResult.checks.length
  return `${passed}/${total} checks passed`
}

export function getVerificationWarnings(): string[] {
  const result = verificationStore.get().lastResult
  if (!result) return []
  return result.checks.filter((c) => c.warning && c.passed).map((c) => c.warning!)
}

export function getVerificationErrors(): string[] {
  const result = verificationStore.get().lastResult
  if (!result) return []
  return result.checks.filter((c) => !c.passed).map((c) => c.error || c.name)
}
