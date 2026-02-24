// SPDX-License-Identifier: Apache-2.0
export const LAMPORTS_PER_SOL = 1_000_000_000
export const SCHEDULER_DEFAULT_LAMPORTS_PER_BILLION_INSTRUCTIONS = 10_000_000
export const SCHEDULER_DEFAULT_FLAT_FEE_LAMPORTS = 0
export const SCHEDULER_DEFAULT_REDUNDANCY_MULTIPLIER = 3
export const RUN_DEFAULT_ESCROW_FLOOR_LAMPORTS = 1_000_000
export const PROGRAM_DEFAULT_PROTOCOL_FEE_BPS = 100

export type CommitteeTier = {
  name: string
  minEscrowLamports: number
  maxEscrowLamportsExclusive: number | null
  committeeSize: number
  quorum: number
}

const INSTRUCTION_PRICE_QUANTUM = 1_000_000_000n

export const COMMITTEE_TIERS: CommitteeTier[] = [
  {
    name: 'Tier 0',
    minEscrowLamports: 0,
    maxEscrowLamportsExclusive: 100_000_000,
    committeeSize: 3,
    quorum: 2
  },
  {
    name: 'Tier 1',
    minEscrowLamports: 100_000_000,
    maxEscrowLamportsExclusive: 1_000_000_000,
    committeeSize: 5,
    quorum: 3
  },
  {
    name: 'Tier 2',
    minEscrowLamports: 1_000_000_000,
    maxEscrowLamportsExclusive: 10_000_000_000,
    committeeSize: 7,
    quorum: 5
  },
  {
    name: 'Tier 3',
    minEscrowLamports: 10_000_000_000,
    maxEscrowLamportsExclusive: null,
    committeeSize: 9,
    quorum: 6
  }
]

export function lamportsToSol(lamports: number): number {
  return lamports / LAMPORTS_PER_SOL
}

export function requiredInstructionEscrowLamports(
  maxInstructions: number,
  lamportsPerBillionInstructions: number,
  redundancyMultiplier: number,
  flatFeeLamports: number
): number {
  const instructions = BigInt(Math.max(0, maxInstructions))
  const pricePerBillion = BigInt(Math.max(0, lamportsPerBillionInstructions))
  const redundancy = BigInt(Math.max(1, redundancyMultiplier))
  const flatFee = BigInt(Math.max(0, flatFeeLamports))
  const usage = instructions * pricePerBillion * redundancy
  const variable = (usage + (INSTRUCTION_PRICE_QUANTUM - 1n)) / INSTRUCTION_PRICE_QUANTUM
  const total = variable + flatFee
  return total > BigInt(Number.MAX_SAFE_INTEGER) ? Number.MAX_SAFE_INTEGER : Number(total)
}

export function committeeTierForEscrow(escrowLamports: number): CommitteeTier {
  const safeEscrow = Math.max(0, escrowLamports)
  for (const tier of COMMITTEE_TIERS) {
    if (safeEscrow < tier.minEscrowLamports) continue
    if (tier.maxEscrowLamportsExclusive === null || safeEscrow < tier.maxEscrowLamportsExclusive) {
      return tier
    }
  }
  return COMMITTEE_TIERS[COMMITTEE_TIERS.length - 1]!
}

export function requiredLockForJobLamports(escrowLamports: number, committeeSize: number): number {
  if (committeeSize <= 0) return 0
  return Math.floor((Math.max(0, escrowLamports) * 3) / (2 * committeeSize))
}

export function computeFinalizePayouts(
  escrowLamports: number,
  protocolFeeBps: number,
  winners: number
): { protocolFeeLamports: number; payoutEachLamports: number; payoutRemainderLamports: number } {
  const safeEscrow = Math.max(0, escrowLamports)
  const safeFeeBps = Math.max(0, protocolFeeBps)
  const safeWinners = Math.max(1, winners)
  const protocolFeeLamports = Math.floor((safeEscrow * safeFeeBps) / 10_000)
  const workerPoolLamports = Math.max(0, safeEscrow - protocolFeeLamports)
  const payoutEachLamports = Math.floor(workerPoolLamports / safeWinners)
  const payoutRemainderLamports = workerPoolLamports - payoutEachLamports * safeWinners
  return { protocolFeeLamports, payoutEachLamports, payoutRemainderLamports }
}
