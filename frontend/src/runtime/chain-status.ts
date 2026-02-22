import { readRuntimeRpcConfig, RPC_CONFIG_EVENT } from '../../lib/solana-config'
import { getConfiguredProgramCount, getConfiguredProgramIds } from '../../lib/solana-deployments'

type RpcEnvelope<T> = {
  jsonrpc: string
  id: number
  result?: T
  error?: { code: number; message: string }
}

let initialized = false
let chainDataLoading = false
let deploymentStatusLoading = false
let chainDataLastUpdateMs = 0
let deploymentStatusLastUpdateMs = 0
const CHAIN_DATA_REFRESH_DEBOUNCE_MS = 2500

function setField(name: string, value: string): void {
  const els = document.querySelectorAll<HTMLElement>(`[data-chain-field="${name}"]`)
  for (const el of els) el.textContent = value
}

function setText(selector: string, value: string): void {
  const el = document.querySelector<HTMLElement>(selector)
  if (el) el.textContent = value
}

function formatInt(value: number): string {
  return new Intl.NumberFormat('en-US').format(value)
}

function formatSol(lamports: number): string {
  return `${(lamports / 1_000_000_000).toLocaleString('en-US', { maximumFractionDigits: 4 })} SOL`
}

async function rpcCall<T>(rpcUrl: string, method: string, params: unknown[] = []): Promise<T> {
  const res = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: Date.now(),
      method,
      params
    })
  })

  if (!res.ok) throw new Error(`rpc_http_${res.status}`)
  const payload = (await res.json()) as RpcEnvelope<T>
  if (payload.error) throw new Error(payload.error.message)
  if (payload.result === undefined) throw new Error('rpc_no_result')
  return payload.result
}

async function loadChainData(): Promise<void> {
  if (chainDataLoading) return
  const now = Date.now()
  if (now - chainDataLastUpdateMs < CHAIN_DATA_REFRESH_DEBOUNCE_MS) return
  if (!document.querySelector('[data-chain-field]')) return
  const cfg = readRuntimeRpcConfig()
  if (!cfg?.rpcUrl) return

  chainDataLoading = true
  setField('cluster', cfg.cluster || 'unknown')
  setField('rpcUrl', cfg.rpcUrl || 'unknown')

  try {
    const [slot, blockHeight, epochInfo, perf, supply] = await Promise.all([
      rpcCall<number>(cfg.rpcUrl, 'getSlot', []),
      rpcCall<number>(cfg.rpcUrl, 'getBlockHeight', []),
      rpcCall<{ epoch: number }>(cfg.rpcUrl, 'getEpochInfo', []),
      rpcCall<Array<{ numTransactions: number; samplePeriodSecs: number }>>(
        cfg.rpcUrl,
        'getRecentPerformanceSamples',
        [1]
      ),
      rpcCall<{ value: { total: number } }>(cfg.rpcUrl, 'getSupply', [])
    ])

    setField('slot', formatInt(slot))
    setField('blockHeight', formatInt(blockHeight))
    setField('epoch', formatInt(epochInfo.epoch))
    setField('supplySol', formatSol(supply.value.total))

    const sample = perf[0]
    if (sample && sample.samplePeriodSecs > 0) {
      setField('tps', (sample.numTransactions / sample.samplePeriodSecs).toFixed(2))
    } else {
      setField('tps', 'n/a')
    }

    if (cfg.treasuryAccount) {
      const balance = await rpcCall<{ value: number }>(cfg.rpcUrl, 'getBalance', [cfg.treasuryAccount, { commitment: 'confirmed' }])
      setField('treasurySol', formatSol(balance.value))
    } else {
      setField('treasurySol', 'not configured')
    }
  } catch {
    const fallback = 'rpc unavailable'
    setField('slot', fallback)
    setField('blockHeight', fallback)
    setField('epoch', fallback)
    setField('tps', fallback)
    setField('supplySol', fallback)
    setField('treasurySol', fallback)
  } finally {
    chainDataLoading = false
    chainDataLastUpdateMs = Date.now()
  }
}

async function isExecutableProgram(rpcUrl: string, programId: string): Promise<boolean> {
  const info = await rpcCall<{ value: { executable?: boolean } | null }>(rpcUrl, 'getAccountInfo', [
    programId,
    { commitment: 'confirmed', encoding: 'base64' }
  ])
  return Boolean(info.value?.executable)
}

async function loadDeploymentStatus(): Promise<void> {
  if (deploymentStatusLoading) return
  const now = Date.now()
  if (now - deploymentStatusLastUpdateMs < CHAIN_DATA_REFRESH_DEBOUNCE_MS) return
  if (!document.querySelector('[data-deployment-badge]') && !document.querySelector('[data-deployment-detail]')) return

  const cfg = readRuntimeRpcConfig()
  const cluster = cfg.cluster || 'unknown'
  const rpcUrl = cfg.rpcUrl || ''
  const configuredCount = getConfiguredProgramCount(cluster)
  const badgePrefix = cluster === 'localnet' ? 'Live on Localnet' : `Cluster: ${cluster}`

  if (!configuredCount) {
    setText('[data-deployment-badge]', `${badgePrefix} (No deployment)`)
    setText('[data-deployment-detail]', `No program IDs configured for ${cluster} yet.`)
    return
  }
  if (!rpcUrl) {
    setText('[data-deployment-badge]', `${badgePrefix} (RPC unavailable)`)
    setText('[data-deployment-detail]', `Configured program IDs: ${configuredCount}. Live verification requires RPC connectivity.`)
    return
  }

  deploymentStatusLoading = true
  try {
    const ids = getConfiguredProgramIds(cluster)
    const checks = await Promise.all(ids.map((id: string) => isExecutableProgram(rpcUrl, id)))
    const liveCount = checks.filter(Boolean).length
    const isLive = liveCount > 0
    setText('[data-deployment-badge]', isLive ? `${badgePrefix} (${liveCount}/${configuredCount} live)` : `${badgePrefix} (Not deployed)`)
    setText('[data-deployment-detail]', `Program deployments verified on ${cluster} via RPC: ${liveCount} of ${configuredCount} configured IDs.`)
  } catch {
    setText('[data-deployment-badge]', `${badgePrefix} (Verification unavailable)`)
    setText('[data-deployment-detail]', `Configured program IDs: ${configuredCount}. Could not verify deployments against current RPC endpoint.`)
  } finally {
    deploymentStatusLoading = false
    deploymentStatusLastUpdateMs = Date.now()
  }
}

function hasChainWidgets(): boolean {
  return Boolean(document.querySelector('[data-chain-field], [data-deployment-badge], [data-deployment-detail]'))
}

export async function initChainStatusWidgets(): Promise<void> {
  if (!hasChainWidgets()) return

  if (!initialized) {
    initialized = true
    window.addEventListener(RPC_CONFIG_EVENT, () => {
      void loadChainData()
      void loadDeploymentStatus()
    })
  }

  await Promise.all([loadChainData(), loadDeploymentStatus()])
}
