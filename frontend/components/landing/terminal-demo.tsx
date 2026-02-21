import { For, createSignal } from 'solid-js'
import { Card } from '../ui/card'
import { Button } from '../ui/button'
import { sha512 } from '@noble/hashes/sha2'
import * as ed25519 from '@noble/ed25519'
import bs58 from 'bs58'

const DOMAIN_TAG = new TextEncoder().encode('edgerun.solana.addressgen.v1')
const DEFAULT_DEMO_COMMAND =
  'address local --seed-hex 0707070707070707070707070707070707070707070707070707070707070707 --prefix So --start 0 --end 50000'

type SearchOutcome =
  | { kind: 'found'; counter: number; address: string; pubkeyHex: string; keypairHex: string; attempts: number; durationMs: number }
  | { kind: 'not_found'; attempts: number; durationMs: number }

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}

function hexToBytes(input: string): Uint8Array {
  const clean = input.trim().toLowerCase()
  if (!/^[0-9a-f]+$/.test(clean)) throw new Error('seed_hex must be hex')
  if (clean.length !== 64) throw new Error('seed_hex must decode to 32 bytes')
  const out = new Uint8Array(32)
  for (let i = 0; i < 32; i += 1) {
    out[i] = Number.parseInt(clean.slice(i * 2, i * 2 + 2), 16)
  }
  return out
}

function encodeCounterLE(counter: number): Uint8Array {
  const out = new Uint8Array(8)
  let x = BigInt(counter)
  for (let i = 0; i < 8; i += 1) {
    out[i] = Number(x & 0xffn)
    x >>= 8n
  }
  return out
}

async function derive(seed: Uint8Array, counter: number): Promise<{ pubkey: Uint8Array; address: string; keypair: Uint8Array }> {
  const preimage = new Uint8Array(DOMAIN_TAG.length + seed.length + 8)
  preimage.set(DOMAIN_TAG, 0)
  preimage.set(seed, DOMAIN_TAG.length)
  preimage.set(encodeCounterLE(counter), DOMAIN_TAG.length + seed.length)
  const digest = sha512(preimage)
  const privateKey = digest.slice(0, 32)
  const pubkey = await ed25519.getPublicKeyAsync(privateKey)
  const keypair = new Uint8Array(64)
  keypair.set(privateKey, 0)
  keypair.set(pubkey, 32)
  return { pubkey, address: bs58.encode(pubkey), keypair }
}

async function searchLocal(
  seed: Uint8Array,
  prefix: string,
  start: number,
  end: number,
  onProgress: (attempts: number) => void
): Promise<SearchOutcome> {
  const started = performance.now()
  let attempts = 0
  for (let counter = start; counter < end; counter += 1) {
    const kp = await derive(seed, counter)
    attempts += 1
    if (kp.address.startsWith(prefix)) {
      return {
        kind: 'found',
        counter,
        address: kp.address,
        pubkeyHex: bytesToHex(kp.pubkey),
        keypairHex: bytesToHex(kp.keypair),
        attempts,
        durationMs: performance.now() - started
      }
    }
    if (attempts % 200 === 0) {
      onProgress(attempts)
      await new Promise<void>((resolve) => window.setTimeout(resolve, 0))
    }
  }
  return { kind: 'not_found', attempts, durationMs: performance.now() - started }
}

function parseArgs(cmd: string): Record<string, string> {
  const tokens = cmd.trim().split(/\s+/)
  const map: Record<string, string> = {}
  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i]
    if (!token) continue
    if (!token.startsWith('--')) continue
    const key = token.slice(2)
    const value = tokens[i + 1]
    if (value && !value.startsWith('--')) {
      map[key] = value
      i += 1
    } else {
      map[key] = 'true'
    }
  }
  return map
}

export function TerminalDemo() {
  const [lines, setLines] = createSignal<string[]>([
    'edgerun browser terminal',
    'type: help',
    `example: ${DEFAULT_DEMO_COMMAND}`
  ])
  const [input, setInput] = createSignal('')
  const [running, setRunning] = createSignal(false)

  const append = (line: string): void => {
    setLines((prev) => [...prev, line])
  }

  const runCommand = async (raw: string): Promise<void> => {
    const cmd = raw.trim()
    if (!cmd) return
    append(`$ ${cmd}`)

    if (cmd === 'help') {
      append('commands:')
      append('  help')
      append('  clear')
      append('  demo')
      append('  address local --seed-hex <64hex> --prefix <base58_prefix> --start <n> --end <n>')
      return
    }
    if (cmd === 'clear') {
      setLines([])
      return
    }
    if (cmd === 'demo') {
      await runCommand(DEFAULT_DEMO_COMMAND)
      return
    }
    if (!cmd.startsWith('address local')) {
      append('unknown command (try: help)')
      return
    }

    try {
      const args = parseArgs(cmd)
      const seed = hexToBytes(args['seed-hex'] || '')
      const prefix = args.prefix || ''
      const start = Number.parseInt(args.start || '0', 10)
      const end = Number.parseInt(args.end || '0', 10)
      if (!prefix) throw new Error('prefix is required')
      if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
        throw new Error('end must be greater than start')
      }

      setRunning(true)
      append(`searching prefix="${prefix}" range=[${start}, ${end})`)
      const result = await searchLocal(seed, prefix, start, end, (attempts) => {
        append(`progress attempts=${attempts}`)
      })
      if (result.kind === 'found') {
        append(`FOUND counter=${result.counter} address=${result.address}`)
        append(
          JSON.stringify(
            {
              status: 'found',
              counter: result.counter,
              address: result.address,
              pubkey_hex: result.pubkeyHex,
              keypair_hex: result.keypairHex,
              attempts: result.attempts,
              duration_ms: Math.round(result.durationMs)
            },
            null,
            2
          )
        )
      } else {
        append(
          JSON.stringify(
            {
              status: 'exhausted_range',
              attempts: result.attempts,
              duration_ms: Math.round(result.durationMs)
            },
            null,
            2
          )
        )
      }
    } catch (err) {
      append(`error: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setRunning(false)
    }
  }

  const onSubmit = async (event: Event): Promise<void> => {
    event.preventDefault()
    if (running()) return
    const current = input()
    setInput('')
    await runCommand(current)
  }

  return (
    <section class="bg-card/30 py-20">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto mb-12 max-w-3xl text-center">
          <h2 class="mb-4 text-3xl font-bold md:text-4xl">Get Started</h2>
          <p class="text-lg text-muted-foreground">
            Run real browser-side bulk Solana address generation here first. Then move to distributed workflows with the same command model.
          </p>
        </div>
        <Card class="mx-auto max-w-4xl overflow-hidden border-primary/20 bg-black">
          <div class="flex items-center justify-between border-b border-border bg-muted/10 px-4 py-3">
            <div class="flex gap-2">
              <span class="h-3 w-3 rounded-full bg-destructive" />
              <span class="h-3 w-3 rounded-full bg-primary" />
              <span class="h-3 w-3 rounded-full bg-accent" />
            </div>
            <span class="text-xs font-mono text-muted-foreground">browser terminal (local mode)</span>
            <Button
              size="sm"
              variant="ghost"
              disabled={running()}
              onClick={() => {
                if (running()) return
                void runCommand('demo')
              }}
            >
              run demo
            </Button>
          </div>
          <div class="h-[500px] overflow-y-auto p-4 font-mono text-sm text-foreground" id="address-terminal-root">
            <For each={lines()}>{(line) => <p class="mb-2 whitespace-pre-wrap break-all">{line}</p>}</For>
            <form class="mt-4 flex items-center gap-2" onSubmit={(e) => void onSubmit(e)}>
              <span class="text-primary">$</span>
              <input
                class="w-full rounded border border-border bg-black/40 px-2 py-1 text-foreground outline-none focus:border-primary"
                value={input()}
                onInput={(e) => setInput(e.currentTarget.value)}
                placeholder="help"
                disabled={running()}
              />
            </form>
          </div>
        </Card>
      </div>
    </section>
  )
}
