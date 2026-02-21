import { createMemo, createSignal, For, onCleanup, onMount } from 'solid-js'
import { Button } from '../ui/button'
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { Select } from '../ui/select'
import { RPC_CONFIG_EVENT, RPC_DEFAULT_BY_CLUSTER, readRuntimeRpcConfig, writeRuntimeRpcConfig } from '../../lib/solana-config'

type WalletProvider = {
  isConnected?: boolean
  publicKey?: { toString(): string }
  connect(opts?: { onlyIfTrusted?: boolean }): Promise<{ publicKey?: { toString(): string } } | void>
  disconnect(): Promise<void>
  on?(event: string, listener: (arg?: any) => void): void
  removeListener?(event: string, listener: (arg?: any) => void): void
}

type ProviderCandidate = {
  name: string
  installUrl: string
  provider: WalletProvider | null
}

declare global {
  interface Window {
    solana?: WalletProvider & { isPhantom?: boolean; isSolflare?: boolean }
    phantom?: { solana?: WalletProvider }
    solflare?: WalletProvider
  }
}

const CLUSTER_OPTIONS = [
  { value: 'localnet', label: 'Localnet' },
  { value: 'devnet', label: 'Devnet' },
  { value: 'testnet', label: 'Testnet' },
  { value: 'mainnet-beta', label: 'Mainnet Beta' },
  { value: 'custom', label: 'Custom RPC' }
] as const

function getCandidates(): ProviderCandidate[] {
  if (typeof window === 'undefined') return []
  const phantom = window.phantom?.solana || (window.solana?.isPhantom ? window.solana : null)
  const solflare = window.solflare || (window.solana?.isSolflare ? window.solana : null)
  return [
    { name: 'Phantom', installUrl: 'https://phantom.app/', provider: phantom || null },
    { name: 'Solflare', installUrl: 'https://solflare.com/', provider: solflare || null }
  ]
}

function shorten(address: string): string {
  if (address.length < 12) return address
  return `${address.slice(0, 4)}...${address.slice(-4)}`
}

function inferClusterFromRpcUrl(rpcUrl: string): string {
  for (const [cluster, knownUrl] of Object.entries(RPC_DEFAULT_BY_CLUSTER)) {
    if (knownUrl === rpcUrl) return cluster
  }
  return 'custom'
}

export function WalletButton() {
  const [mounted, setMounted] = createSignal(false)
  const [providerName, setProviderName] = createSignal('')
  const [address, setAddress] = createSignal('')
  const [balance, setBalance] = createSignal('')
  const [connecting, setConnecting] = createSignal(false)
  const [error, setError] = createSignal('')
  const [dialogOpen, setDialogOpen] = createSignal(false)
  const currentRpc = readRuntimeRpcConfig()
  const initialCluster = inferClusterFromRpcUrl(currentRpc.rpcUrl)
  const [cluster, setCluster] = createSignal(initialCluster)
  const [customRpcUrl, setCustomRpcUrl] = createSignal(initialCluster === 'custom' ? currentRpc.rpcUrl : '')
  const [activeRpcUrl, setActiveRpcUrl] = createSignal(currentRpc.rpcUrl)
  let activeProvider: WalletProvider | null = null

  const available = createMemo(() => getCandidates())
  const installed = createMemo(() => available().find((item) => item.provider) || null)
  const connected = createMemo(() => Boolean(address()))

  async function refreshBalance(pubkey: string): Promise<void> {
    const rpcUrl = readRuntimeRpcConfig().rpcUrl || 'https://api.devnet.solana.com'
    setActiveRpcUrl(rpcUrl)
    try {
      const res = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: Date.now(), method: 'getBalance', params: [pubkey, { commitment: 'confirmed' }] })
      })
      const payload = await res.json() as { result?: { value: number } }
      const lamports = payload.result?.value
      if (typeof lamports !== 'number') return
      setBalance(`${(lamports / 1_000_000_000).toLocaleString('en-US', { maximumFractionDigits: 3 })} SOL`)
    } catch {
      setBalance('')
    }
  }

  function bindProvider(provider: WalletProvider): void {
    const onDisconnect = () => {
      setAddress('')
      setBalance('')
      setError('')
    }
    const onAccount = (next?: { toString(): string }) => {
      if (!next) {
        onDisconnect()
        return
      }
      const value = next.toString()
      setAddress(value)
      void refreshBalance(value)
    }
    provider.on?.('disconnect', onDisconnect)
    provider.on?.('accountChanged', onAccount)
    onCleanup(() => {
      provider.removeListener?.('disconnect', onDisconnect)
      provider.removeListener?.('accountChanged', onAccount)
    })
  }

  async function connect(): Promise<void> {
    setError('')
    if (connecting()) return
    const target = installed()
    if (!target?.provider) {
      window.open(target?.installUrl || 'https://phantom.app/', '_blank', 'noopener,noreferrer')
      return
    }
    setConnecting(true)
    setProviderName(target.name)
    activeProvider = target.provider
    try {
      const result = await target.provider.connect()
      const pubkey = result?.publicKey?.toString() || target.provider.publicKey?.toString() || ''
      if (!pubkey) throw new Error('wallet_public_key_missing')
      setAddress(pubkey)
      void refreshBalance(pubkey)
      bindProvider(target.provider)
      setDialogOpen(true)
    } catch {
      setError('Connection failed')
    } finally {
      setConnecting(false)
    }
  }

  async function disconnect(): Promise<void> {
    setError('')
    try {
      await activeProvider?.disconnect()
    } catch {
      // ignore provider disconnect failure
    }
    setAddress('')
    setBalance('')
  }

  function applyRpcSelection(): void {
    const selected = cluster()
    const rpcUrl = selected === 'custom'
      ? customRpcUrl().trim()
      : (RPC_DEFAULT_BY_CLUSTER[selected] || RPC_DEFAULT_BY_CLUSTER.devnet)
    if (!rpcUrl) return
    const merged = writeRuntimeRpcConfig({ cluster: selected, rpcUrl })
    setActiveRpcUrl(merged.rpcUrl)
    setError('')
    if (address()) void refreshBalance(address())
  }

  onMount(() => {
    setMounted(true)
    const target = installed()
    const cfg = readRuntimeRpcConfig()
    setActiveRpcUrl(cfg.rpcUrl)
    const inferred = inferClusterFromRpcUrl(cfg.rpcUrl)
    setCluster(inferred)
    setCustomRpcUrl(inferred === 'custom' ? cfg.rpcUrl : '')

    if (!target?.provider) return
    activeProvider = target.provider
    setProviderName(target.name)
    const pubkey = target.provider.publicKey?.toString() || ''
    if (target.provider.isConnected && pubkey) {
      setAddress(pubkey)
      void refreshBalance(pubkey)
    }
    bindProvider(target.provider)

    const onRpcChanged = () => {
      const next = readRuntimeRpcConfig()
      setActiveRpcUrl(next.rpcUrl)
      const nextCluster = inferClusterFromRpcUrl(next.rpcUrl)
      setCluster(nextCluster)
      setCustomRpcUrl(nextCluster === 'custom' ? next.rpcUrl : '')
      if (address()) void refreshBalance(address())
    }
    window.addEventListener(RPC_CONFIG_EVENT, onRpcChanged)
    onCleanup(() => window.removeEventListener(RPC_CONFIG_EVENT, onRpcChanged))
  })

  const buttonLabel = createMemo(() => {
    if (!mounted()) return 'Wallet'
    if (connecting()) return 'Connecting...'
    if (connected()) {
      const base = shorten(address())
      return balance() ? `${base} • ${balance()}` : base
    }
    if (!installed()) return 'Install Wallet'
    return providerName() ? `Connect ${providerName()}` : 'Connect Wallet'
  })

  return (
    <>
      <Button
        variant="outline"
        size="sm"
        onClick={() => setDialogOpen(true)}
        aria-label="Open wallet details"
        title={error() || (connected() ? `Connected: ${address()}` : installed() ? 'Connect wallet' : 'Install Phantom or Solflare')}
      >
        {buttonLabel()}
      </Button>

      <Dialog open={dialogOpen()} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Wallet + Network</DialogTitle>
            <DialogDescription>Manage wallet session and selected Solana cluster/RPC.</DialogDescription>
          </DialogHeader>

          <div class="space-y-3">
            <div class="rounded-lg border border-border p-3 text-sm">
              <p class="font-medium text-foreground">{providerName() || 'Wallet Provider'}</p>
              <p class="font-mono text-xs text-muted-foreground break-all">{address() || 'Not connected'}</p>
              <p class="mt-1 text-xs text-muted-foreground">{balance() || 'Balance unavailable'}</p>
            </div>

            <div class="space-y-1">
              <Label for="wallet-cluster">Cluster</Label>
                <Select
                  id="wallet-cluster"
                  value={cluster()}
                  onChange={(event: Event) => setCluster((event.currentTarget as HTMLSelectElement).value)}
                >
                <For each={CLUSTER_OPTIONS}>{(option) => <option value={option.value}>{option.label}</option>}</For>
              </Select>
            </div>

            {cluster() === 'custom' && (
              <div class="space-y-1">
                <Label for="wallet-custom-rpc">Custom RPC URL</Label>
                <Input
                  id="wallet-custom-rpc"
                  value={customRpcUrl()}
                  placeholder="https://..."
                  class="font-mono text-xs"
                  onInput={(event: Event) => setCustomRpcUrl((event.currentTarget as HTMLInputElement).value)}
                />
              </div>
            )}

            <p class="text-xs text-muted-foreground break-all">Active RPC: {activeRpcUrl()}</p>
          </div>

          <DialogFooter class="flex-wrap justify-between gap-2">
            <div class="flex gap-2">
              {!connected() && installed() && <Button size="sm" onClick={() => void connect()}>{connecting() ? 'Connecting...' : `Connect ${providerName() || 'Wallet'}`}</Button>}
              {!connected() && !installed() && <Button size="sm" onClick={() => window.open('https://phantom.app/', '_blank', 'noopener,noreferrer')}>Install Wallet</Button>}
              <Button size="sm" variant="outline" onClick={applyRpcSelection}>Apply Network</Button>
              {connected() && <Button size="sm" variant="destructive" onClick={() => void disconnect()}>Disconnect</Button>}
            </div>
            <DialogClose class="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm hover:bg-muted/50">
              Close
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
