export type BlogPost = {
  slug: string
  title: string
  excerpt: string
  publishedAt: string
  readingTime: number
  tags: string[]
  author: {
    name: string
    role: string
    bio?: string
  }
}

export type JobResult = {
  workerId: string
  workerName: string
  status: 'completed' | 'failed'
  runtimeMs: number
  outputHash: string
  gasUsed: number
  timestamp: string
}

export type JobRecord = {
  id: string
  name: string
  status: 'queued' | 'running' | 'completed' | 'failed'
  createdAt: string
  wasmHash: string
  runtimeMs: number
  gasUsed: number
  executorCount: number
  consensusReached: boolean
  settlementTx?: string
  input: { fileName: string; sizeBytes: number }
  results: JobResult[]
}

export type TimelineEvent = {
  id: string
  title: string
  description: string
  timestamp: string
  type: 'submitted' | 'running' | 'completed' | 'settled' | 'failed'
  txHash?: string
}

export const blogPosts: BlogPost[] = [
  {
    slug: 'introducing-edgerun',
    title: 'Introducing Edgerun',
    excerpt: 'Deterministic compute on Solana with stake-backed verification and practical developer workflows.',
    publishedAt: '2026-02-18T12:00:00Z',
    readingTime: 8,
    tags: ['Protocol', 'Launch', 'Solana'],
    author: {
      name: 'Edgerun Core Team',
      role: 'Protocol Engineering',
      bio: 'Building dependable compute with deterministic execution and on-chain settlement.'
    }
  }
]

export const jobs: JobRecord[] = [
  {
    id: 'job_8x4k9p2m',
    name: 'solana-address-prefix-search',
    status: 'completed',
    createdAt: '2026-02-21T08:10:00Z',
    wasmHash: '0x8f2a5e6d7c8b1a9f3d2c4e5f61728394aa55bb66cc77dd88ee99ff0011223344',
    runtimeMs: 12240,
    gasUsed: 947201,
    executorCount: 5,
    consensusReached: true,
    settlementTx: '5jXvKf2mB9rwM3Gm1Jx88a4Czv1L4m9f1xJQ8p5u9M5JwF3p5QvG8x9UyQh7J7gM',
    input: { fileName: 'address-request.json', sizeBytes: 2419 },
    results: [
      {
        workerId: 'worker-001',
        workerName: 'atlas-01',
        status: 'completed',
        runtimeMs: 11780,
        outputHash: '0x90ab12cd34ef56789012ab34cd56ef789012ab34cd56ef789012ab34cd56ef78',
        gasUsed: 189201,
        timestamp: '2026-02-21T08:10:12Z'
      },
      {
        workerId: 'worker-002',
        workerName: 'boreal-02',
        status: 'completed',
        runtimeMs: 12240,
        outputHash: '0x90ab12cd34ef56789012ab34cd56ef789012ab34cd56ef789012ab34cd56ef78',
        gasUsed: 190884,
        timestamp: '2026-02-21T08:10:12Z'
      },
      {
        workerId: 'worker-003',
        workerName: 'cinder-03',
        status: 'completed',
        runtimeMs: 12114,
        outputHash: '0x90ab12cd34ef56789012ab34cd56ef789012ab34cd56ef789012ab34cd56ef78',
        gasUsed: 188771,
        timestamp: '2026-02-21T08:10:12Z'
      }
    ]
  }
]

export const timelineEvents: TimelineEvent[] = [
  {
    id: 'evt-1',
    title: 'Job Submitted',
    description: 'Scheduler accepted payload and escrow envelope.',
    timestamp: '2026-02-21T08:10:01Z',
    type: 'submitted'
  },
  {
    id: 'evt-2',
    title: 'Committee Assigned',
    description: 'Workers selected and execution chunk broadcast.',
    timestamp: '2026-02-21T08:10:03Z',
    type: 'running'
  },
  {
    id: 'evt-3',
    title: 'Consensus Reached',
    description: 'Matching deterministic outputs finalized.',
    timestamp: '2026-02-21T08:10:12Z',
    type: 'completed'
  },
  {
    id: 'evt-4',
    title: 'Settlement Finalized',
    description: 'Payout and accounting posted to Solana.',
    timestamp: '2026-02-21T08:10:16Z',
    type: 'settled',
    txHash: '5jXvKf2mB9rwM3Gm1Jx88a4Czv1L4m9f1xJQ8p5u9M5JwF3p5QvG8x9UyQh7J7gM'
  }
]

export function formatDate(value: string): string {
  return new Date(value).toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' })
}

export function formatShortDate(value: string): string {
  return new Date(value).toLocaleDateString('en-US', { dateStyle: 'medium' })
}

export function formatMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

export function formatNumber(value: number): string {
  return new Intl.NumberFormat('en-US').format(value)
}

export function formatHash(value: string, left = 10, right = 8): string {
  if (value.length <= left + right + 3) return value
  return `${value.slice(0, left)}...${value.slice(-right)}`
}

export function jobStatusBadge(status: JobRecord['status']): 'default' | 'secondary' | 'destructive' {
  if (status === 'completed') return 'default'
  if (status === 'failed') return 'destructive'
  return 'secondary'
}
