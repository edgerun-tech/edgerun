// SPDX-License-Identifier: Apache-2.0
import { readFileSync } from 'node:fs'
import path from 'node:path'

const projectRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..')
const deploymentsPath = path.join(projectRoot, 'config', 'solana-deployments.json')

const allowedClusters = new Set(['localnet', 'devnet', 'testnet', 'mainnet-beta'])

function requiredEnv(name, value, errors) {
  if (!value || !value.trim()) {
    errors.push(`Missing required env var: ${name}`)
  }
}

function assertHttps(name, value, errors) {
  if (!value) return
  if (!/^https:\/\//.test(value)) {
    errors.push(`${name} must be an https URL: received "${value}"`)
  }
}

const version = process.env.EDGERUN_VERSION?.trim() || ''
const buildNumber = process.env.EDGERUN_BUILD_NUMBER?.trim() || ''
const siteUrl = process.env.EDGERUN_SITE_URL?.trim() || ''
const siteDomain = process.env.EDGERUN_SITE_DOMAIN?.trim() || ''
const cluster = process.env.SOLANA_CLUSTER?.trim() || ''
const rpcUrl = process.env.SOLANA_RPC_URL?.trim() || ''
const treasuryAccount = process.env.EDGERUN_TREASURY_ACCOUNT?.trim() || ''

const errors = []

requiredEnv('EDGERUN_VERSION', version, errors)
requiredEnv('EDGERUN_BUILD_NUMBER', buildNumber, errors)
requiredEnv('EDGERUN_SITE_URL', siteUrl, errors)
requiredEnv('EDGERUN_SITE_DOMAIN', siteDomain, errors)
requiredEnv('SOLANA_CLUSTER', cluster, errors)
requiredEnv('SOLANA_RPC_URL', rpcUrl, errors)

if (siteUrl) assertHttps('EDGERUN_SITE_URL', siteUrl, errors)
if (siteUrl && siteDomain) {
  try {
    const hostname = new URL(siteUrl).hostname
    if (hostname !== siteDomain) {
      errors.push(`EDGERUN_SITE_DOMAIN "${siteDomain}" must match EDGERUN_SITE_URL hostname "${hostname}"`)
    }
  } catch {
    errors.push(`EDGERUN_SITE_URL is not a valid URL: "${siteUrl}"`)
  }
}

if (cluster && !allowedClusters.has(cluster)) {
  errors.push(`Unsupported SOLANA_CLUSTER "${cluster}". Allowed: ${Array.from(allowedClusters).join(', ')}`)
}

if (rpcUrl && cluster !== 'localnet') {
  assertHttps('SOLANA_RPC_URL', rpcUrl, errors)
}
if (cluster === 'localnet' && rpcUrl && !/^https?:\/\//.test(rpcUrl)) {
  errors.push(`SOLANA_RPC_URL for localnet must be http(s): received "${rpcUrl}"`)
}

if (cluster === 'mainnet-beta' && !treasuryAccount) {
  errors.push('EDGERUN_TREASURY_ACCOUNT is required when SOLANA_CLUSTER=mainnet-beta')
}

try {
  const parsed = JSON.parse(readFileSync(deploymentsPath, 'utf8'))
  const programs = parsed?.programs && typeof parsed.programs === 'object' ? parsed.programs : {}
  const missingPrograms = []
  for (const [programKey, program] of Object.entries(programs)) {
    const byCluster = program?.programIdByCluster && typeof program.programIdByCluster === 'object'
      ? program.programIdByCluster
      : {}
    const value = typeof byCluster[cluster] === 'string' ? byCluster[cluster].trim() : ''
    if (!value) missingPrograms.push(programKey)
  }
  if (!Object.keys(programs).length) {
    errors.push('frontend/config/solana-deployments.json has no programs configured')
  } else if (cluster && missingPrograms.length) {
    errors.push(`Missing programIdByCluster.${cluster} for programs: ${missingPrograms.join(', ')}`)
  }
} catch (error) {
  errors.push(`Failed to read deployments config at ${deploymentsPath}: ${String(error)}`)
}

if (errors.length) {
  console.error('production build validation failed:')
  for (const error of errors) {
    console.error(`- ${error}`)
  }
  process.exit(1)
}

console.log('production build validation passed')
