export const RPC_CONFIG_EVENT = 'edgerun:rpc-config-changed'
export const RPC_STORAGE_CLUSTER_KEY = 'edgerun.rpc.cluster'
export const RPC_STORAGE_URL_KEY = 'edgerun.rpc.url'

export const RPC_DEFAULT_BY_CLUSTER: Record<string, string> = {
  localnet: 'http://127.0.0.1:8899',
  devnet: 'https://api.devnet.solana.com',
  testnet: 'https://api.testnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com'
}

export type RuntimeRpcConfig = {
  cluster: string
  rpcUrl: string
  treasuryAccount: string
  deployments: Record<string, { label: string; programIdByCluster: Record<string, string> }>
}

declare global {
  interface Window {
    __EDGERUN_RPC_CONFIG?: {
      cluster?: string
      rpcUrl?: string
      treasuryAccount?: string
      deployments?: Record<string, { label: string; programIdByCluster: Record<string, string> }>
    }
  }
}

export function readRuntimeRpcConfig(): RuntimeRpcConfig {
  const devnetRpc = RPC_DEFAULT_BY_CLUSTER.devnet ?? 'https://api.devnet.solana.com'
  if (typeof window === 'undefined') {
    return { cluster: 'devnet', rpcUrl: devnetRpc, treasuryAccount: '', deployments: {} }
  }
  const base = window.__EDGERUN_RPC_CONFIG || {}
  const storedCluster = window.localStorage.getItem(RPC_STORAGE_CLUSTER_KEY)?.trim()
  const storedRpcUrl = window.localStorage.getItem(RPC_STORAGE_URL_KEY)?.trim()
  const cluster = storedCluster || base.cluster || 'devnet'
  const fallbackRpc = RPC_DEFAULT_BY_CLUSTER[cluster] || devnetRpc
  const rpcUrl = storedRpcUrl || base.rpcUrl || fallbackRpc
  const treasuryAccount = base.treasuryAccount || ''
  const deployments = base.deployments || {}
  return { cluster, rpcUrl, treasuryAccount, deployments }
}

export function writeRuntimeRpcConfig(next: { cluster: string; rpcUrl: string }): RuntimeRpcConfig {
  if (typeof window === 'undefined') {
    return { cluster: next.cluster, rpcUrl: next.rpcUrl, treasuryAccount: '', deployments: {} }
  }
  window.localStorage.setItem(RPC_STORAGE_CLUSTER_KEY, next.cluster)
  window.localStorage.setItem(RPC_STORAGE_URL_KEY, next.rpcUrl)
  window.__EDGERUN_RPC_CONFIG = {
    ...(window.__EDGERUN_RPC_CONFIG || {}),
    cluster: next.cluster,
    rpcUrl: next.rpcUrl
  }
  const merged = readRuntimeRpcConfig()
  window.dispatchEvent(new CustomEvent(RPC_CONFIG_EVENT, { detail: merged }))
  return merged
}
