import deploymentsConfig from '../config/solana-deployments.json'

export type SolanaCluster = 'localnet' | 'devnet' | 'testnet' | 'mainnet-beta'

export type ProgramDeploymentConfig = {
  label: string
  programIdByCluster: Record<string, string>
}

export type DeploymentsConfig = {
  programs: Record<string, ProgramDeploymentConfig>
}

const SOLANA_DEPLOYMENTS = deploymentsConfig as DeploymentsConfig

export function getProgramIdForCluster(programKey: string, cluster: string): string {
  const entry = SOLANA_DEPLOYMENTS.programs[programKey]
  if (!entry) return ''
  return (entry.programIdByCluster[cluster] || '').trim()
}

export function getConfiguredProgramIds(cluster: string): string[] {
  const ids = Object.values(SOLANA_DEPLOYMENTS.programs)
    .map((program) => (program.programIdByCluster[cluster] || '').trim())
    .filter(Boolean)
  return Array.from(new Set(ids))
}

export function getConfiguredProgramCount(cluster: string): number {
  return getConfiguredProgramIds(cluster).length
}
