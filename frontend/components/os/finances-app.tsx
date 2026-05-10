"use client"

import * as React from "react"
import {
  ArrowDownLeft,
  ArrowRightLeft,
  ArrowUpRight,
  Check,
  ChevronDown,
  Copy,
  ExternalLink,
  RefreshCw,
  Send,
  Shield,
  TrendingUp,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { formatDate } from "@/lib/format"

type TxType = "send" | "receive" | "earn" | "exchange"

type AssetInfo = {
  symbol: string
  network: string
  contract?: string
}

type QuoteResponse = {
  id: string
  settlement_asset: string
  pay_asset: string
  settlement_amount: string
  pay_amount: string
  rate: string
  expires_at_ms: number
  estimated_seconds?: number
}

type OrderResponse = {
  id: string
  status: string
  deposit_address: string
  settlement_asset: string
  settlement_amount: string
  pay_amount: string
}

type OrderStatusResponse = {
  id: string
  status: string
  canonical_status?: number
  deposit_address: string
  settlement_amount: string
  pay_amount: string
  event_count?: number
  terminal?: boolean
  manual_review_required?: boolean
  last_event_type?: string
}

type Transaction = {
  id: string
  type: TxType
  label: string
  amount: number
  ts: Date
  status: "confirmed" | "pending"
}

const ADDRESS = "edge1qxy2kgdygjrsqtzq2n0yrf249.run"
const SHORT_ADDRESS = "edge1qxy…249.run"

const FALLBACK_ASSETS: AssetInfo[] = [
  { symbol: "USDT", network: "Tron", contract: "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t" },
  { symbol: "BTC", network: "Bitcoin" },
  { symbol: "ETH", network: "Ethereum" },
  { symbol: "DOGE", network: "Dogecoin" },
  { symbol: "SOL", network: "Solana" },
]

const ASSET_COLORS: Record<string, string> = {
  EDGE: "#22c55e",
  USDT: "#26a17b",
  BTC: "#f7931a",
  ETH: "#627eea",
  DOGE: "#c2a633",
  SOL: "#9945ff",
}

const portfolioAssets = [
  { symbol: "EDGE", network: "Edgerun", amount: "134.57", value: 247.61, fill: ASSET_COLORS.EDGE },
  { symbol: "USDT", network: "Tron", amount: "0.00", value: 0, fill: ASSET_COLORS.USDT },
  { symbol: "BTC", network: "Bitcoin", amount: "0.0000", value: 0, fill: ASSET_COLORS.BTC },
  { symbol: "SOL", network: "Solana", amount: "0.00", value: 0, fill: ASSET_COLORS.SOL },
]

type QuoteResponse = {
  id: string
  settlement_asset: string
  pay_asset: string
  settlement_amount: string
  pay_amount: string
  rate: string
  expires_at_ms: number
  estimated_seconds?: number
}

type OrderResponse = {
  id: string
  status: string
  deposit_address: string
  settlement_asset: string
  settlement_amount: string
  pay_amount: string
}

type OrderStatusResponse = {
  id: string
  status: string
  canonical_status?: number
  deposit_address: string
  settlement_amount: string
  pay_amount: string
  event_count?: number
  terminal?: boolean
  manual_review_required?: boolean
  last_event_type?: string
}

type Transaction = {
  id: string
  type: TxType
  label: string
  amount: number
  ts: Date
  status: "confirmed" | "pending"
}

const ADDRESS = "edge1qxy2kgdygjrsqtzq2n0yrf249.run"
const SHORT_ADDRESS = "edge1qxy…249.run"

const FALLBACK_ASSETS: AssetInfo[] = [
  { symbol: "USDT", network: "Tron", contract: "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t" },
  { symbol: "BTC", network: "Bitcoin" },
  { symbol: "ETH", network: "Ethereum" },
  { symbol: "DOGE", network: "Dogecoin" },
  { symbol: "SOL", network: "Solana" },
]

const DEMO_TXS: Transaction[] = [
  { id: "t1", type: "receive", label: "Compute reward", amount: 4.82, ts: new Date(Date.now() - 3600000), status: "confirmed" },
  { id: "t2", type: "send", label: "Paid Elias Voss", amount: -2.0, ts: new Date(Date.now() - 7200000), status: "confirmed" },
  { id: "t3", type: "earn", label: "Node uptime bonus", amount: 1.2, ts: new Date(Date.now() - 86400000), status: "confirmed" },
  { id: "t4", type: "receive", label: "Transfer from Ara", amount: 10.0, ts: new Date(Date.now() - 172800000), status: "confirmed" },
]

const ASSET_COLORS: Record<string, string> = {
  EDGE: "#22c55e",
  USDT: "#26a17b",
  BTC: "#f7931a",
  ETH: "#627eea",
  DOGE: "#c2a633",
  SOL: "#9945ff",
}

const portfolioAssets = [
  { symbol: "EDGE", network: "Edgerun", amount: "134.57", value: 247.61, fill: ASSET_COLORS.EDGE },
  { symbol: "USDT", network: "Tron", amount: "0.00", value: 0, fill: ASSET_COLORS.USDT },
  { symbol: "BTC", network: "Bitcoin", amount: "0.0000", value: 0, fill: ASSET_COLORS.BTC },
  { symbol: "SOL", network: "Solana", amount: "0.00", value: 0, fill: ASSET_COLORS.SOL },
]

function parseAssetId(value: string): AssetInfo {
  const [symbol, network] = value.split(":")
  return { symbol: symbol || "", network: network || "" }
}

function assetId(asset: AssetInfo) {
  return `${asset.symbol}:${asset.network}`
}

function backendNetwork(network: string) {
  return network.toLowerCase()
}

async function readJson<T>(response: Response): Promise<T> {
  const json = await response.json().catch(() => null)
  if (!response.ok || json?.error) throw new Error(json?.error || json?.detail || `HTTP ${response.status}`)
  return json as T
}

function statusTone(status: string) {
  const normalized = status.toLowerCase()
  if (["completed", "refunded"].includes(normalized)) return "text-[var(--status-online)] bg-[var(--status-online)]/10"
  if (["failed", "rejected", "canceled", "expired", "refund_required"].includes(normalized)) return "text-[var(--status-error)] bg-[var(--status-error)]/10"
  if (["action_required", "on_hold", "manual_review_required"].includes(normalized)) return "text-[var(--status-warning)] bg-[var(--status-warning)]/10"
  return "text-primary bg-primary/10"
}

function FinanceCard({ children, className }: { children: React.ReactNode; className?: string }) {
  return <section className={cn("rounded-2xl border border-border bg-card p-4 shadow-sm", className)}>{children}</section>
}

function AssetIcon({ asset, size = "md" }: { asset: AssetInfo | { symbol: string; network?: string }; size?: "sm" | "md" | "lg" }) {
  const color = ASSET_COLORS[asset.symbol.toUpperCase()] || "#64748b"
  const sizeClass = size === "lg" ? "h-10 w-10 text-sm" : size === "sm" ? "h-6 w-6 text-[10px]" : "h-8 w-8 text-xs"
  return (
    <div
      className={cn("flex flex-shrink-0 items-center justify-center rounded-full font-bold text-white shadow-sm ring-1 ring-white/10", sizeClass)}
      style={{ background: `radial-gradient(circle at 30% 20%, rgba(255,255,255,.35), transparent 30%), ${color}` }}
    >
      {asset.symbol.slice(0, 1).toUpperCase()}
    </div>
  )
}

function AssetPicker({ value, assets, onChange }: { value: string; assets: AssetInfo[]; onChange: (value: string) => void }) {
  const [open, setOpen] = React.useState(false)
  const selected = assets.find((asset) => assetId(asset) === value) || parseAssetId(value)

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-2 rounded-full bg-secondary px-2 py-1.5 text-sm font-semibold text-foreground transition hover:bg-secondary/80"
        aria-expanded={open}
        aria-label={`Select asset, currently ${selected.symbol}`}
      >
        <AssetIcon asset={selected} />
        <span>{selected.symbol}</span>
        <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
      </button>
      {open && (
        <div className="absolute right-0 top-11 z-50 w-64 overflow-hidden rounded-xl border border-border bg-popover shadow-xl">
          <div className="border-b border-border px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Select asset</div>
          <div className="max-h-72 overflow-y-auto p-1">
            {assets.map((asset) => (
              <button
                key={assetId(asset)}
                onClick={() => {
                  onChange(assetId(asset))
                  setOpen(false)
                }}
                className={cn("flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left transition hover:bg-secondary", assetId(asset) === value && "bg-primary/10")}
              >
                <AssetIcon asset={asset} />
                <div className="min-w-0 flex-1">
                  <div className="text-sm font-semibold text-foreground">{asset.symbol}</div>
                  <div className="truncate text-[11px] text-muted-foreground">{asset.network}{asset.contract ? ` · ${asset.contract.slice(0, 8)}…` : ""}</div>
                </div>
                {assetId(asset) === value && <Check className="h-4 w-4 text-primary" />}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function TxIcon({ type }: { type: TxType }) {
  const config = {
    send: { icon: <ArrowUpRight className="h-4 w-4" />, color: "text-[var(--status-error)] bg-[var(--status-error)]/10" },
    receive: { icon: <ArrowDownLeft className="h-4 w-4" />, color: "text-[var(--status-online)] bg-[var(--status-online)]/10" },
    earn: { icon: <RefreshCw className="h-4 w-4" />, color: "text-[var(--status-warning)] bg-[var(--status-warning)]/10" },
    exchange: { icon: <ArrowRightLeft className="h-4 w-4" />, color: "text-primary bg-primary/10" },
  }[type]
  return <div className={cn("flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full", config.color)}>{config.icon}</div>
}

function donutGradient(segments: Array<{ value: number; fill: string }>) {
  const total = segments.reduce((acc, segment) => acc + segment.value, 0)
  if (total <= 0) return "var(--muted)"
  let cursor = 0
  return segments
    .map((segment) => {
      const start = cursor
      cursor += (segment.value / total) * 100
      return `${segment.fill} ${start.toFixed(2)}% ${cursor.toFixed(2)}%`
    })
    .join(", ")
}

function AllocationCard({ totalValue }: { totalValue: number }) {
  const chartData = portfolioAssets.filter((asset) => asset.value > 0)
  const total = chartData.reduce((acc, asset) => acc + asset.value, 0)
  return (
    <FinanceCard className="flex flex-col">
      <div className="mb-2 text-center">
        <h3 className="text-sm font-semibold">Allocation</h3>
        <p className="text-xs text-muted-foreground">Current finance workspace</p>
      </div>
      <div className="mx-auto flex h-[210px] w-[210px] items-center justify-center">
        <div
          className="flex h-[164px] w-[164px] items-center justify-center rounded-full"
          style={{ background: `conic-gradient(${donutGradient(chartData)})` }}
        >
          <div className="flex h-[112px] w-[112px] flex-col items-center justify-center rounded-full bg-card">
            <span className="text-xl font-bold text-foreground">${total.toFixed(0)}</span>
            <span className="text-xs text-muted-foreground">Portfolio</span>
          </div>
        </div>
      </div>
      <div className="mt-auto flex items-center justify-center gap-2 text-xs text-muted-foreground">
        <TrendingUp className="h-3.5 w-3.5 text-[var(--status-online)]" />
        EDGE rewards active · ${(totalValue * 0.052).toFixed(2)} projected monthly
      </div>
    </FinanceCard>
  )
}

function SwapAmountPanel({ label, value, onChange, assetValue, assets, onAssetChange, readOnly }: {
  label: string
  value: string
  onChange?: (value: string) => void
  assetValue: string
  assets: AssetInfo[]
  onAssetChange: (value: string) => void
  readOnly?: boolean
}) {
  const asset = parseAssetId(assetValue)
  return (
    <div className="rounded-2xl border border-border bg-background/80 p-4">
      <div className="mb-3 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">{label}</span>
        <span className="text-[10px] text-muted-foreground">{asset.network}</span>
      </div>
      <div className="flex items-center gap-3">
        <input
          value={value}
          onChange={(event) => onChange?.(event.target.value)}
          readOnly={readOnly}
          placeholder="0"
          inputMode="decimal"
          aria-label={label}
          className="min-w-0 flex-1 bg-transparent font-mono text-3xl font-semibold text-foreground outline-none placeholder:text-muted-foreground/30"
        />
        <AssetPicker value={assetValue} assets={assets} onChange={onAssetChange} />
      </div>
    </div>
  )
}

function ExchangeWorkspace({ onExchangeTx }: { onExchangeTx: (tx: Transaction) => void }) {
  const [assets, setAssets] = React.useState<AssetInfo[]>(FALLBACK_ASSETS)
  const [source, setSource] = React.useState("USDT:Tron")
  const [target, setTarget] = React.useState("BTC:Bitcoin")
  const [amount, setAmount] = React.useState("100")
  const [amountSide, setAmountSide] = React.useState<"settlement" | "pay">("settlement")
  const [mode, setMode] = React.useState<"instant" | "floating">("instant")
  const [recipient, setRecipient] = React.useState("")
  const [refund, setRefund] = React.useState("")
  const [quote, setQuote] = React.useState<QuoteResponse | null>(null)
  const [order, setOrder] = React.useState<OrderResponse | null>(null)
  const [orderStatus, setOrderStatus] = React.useState<OrderStatusResponse | null>(null)
  const [loading, setLoading] = React.useState(false)
  const [statusLoading, setStatusLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  const [info, setInfo] = React.useState<string | null>(null)

  React.useEffect(() => {
    let cancelled = false
    async function loadAssets() {
      try {
        const response = await fetch("/api/exchange/v1/assets", { cache: "no-store" })
        const json = await readJson<{ assets: Record<string, { network: string; contract?: string }> }>(response)
        const parsed = Object.entries(json.assets).map(([symbol, value]) => ({ symbol, network: value.network, contract: value.contract }))
        if (!cancelled && parsed.length > 0) setAssets(parsed)
      } catch (err) {
        if (!cancelled) setInfo(`Exchange API unavailable; using static asset catalog. ${err instanceof Error ? err.message : String(err)}`)
      }
    }
    loadAssets()
    return () => { cancelled = true }
  }, [])

  const selectedSource = React.useMemo(() => parseAssetId(source), [source])
  const selectedTarget = React.useMemo(() => parseAssetId(target), [target])
  const quoteExpired = quote ? Date.now() > quote.expires_at_ms : false
  const quoteSecondsLeft = quote ? Math.max(0, Math.floor((quote.expires_at_ms - Date.now()) / 1000)) : 0

  async function requestQuote() {
    setLoading(true)
    setError(null)
    setInfo(null)
    setOrder(null)
    setOrderStatus(null)
    try {
      const response = await fetch("/api/exchange/v1/quote", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          settlement: { symbol: selectedSource.symbol, network: backendNetwork(selectedSource.network) },
          pay: { symbol: selectedTarget.symbol, network: backendNetwork(selectedTarget.network) },
          amount,
          mode,
          amount_side: amountSide,
          refund_address: refund || undefined,
          recipient_address: recipient || undefined,
        }),
      })
      const json = await readJson<QuoteResponse>(response)
      setQuote(json)
      setInfo(`Quote ${json.id} received. Rate ${json.rate}.`)
    } catch (err) {
      setQuote(null)
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  async function refreshStatus(id = order?.id) {
    if (!id) return
    setStatusLoading(true)
    setError(null)
    try {
      const response = await fetch(`/api/exchange/v1/order/${encodeURIComponent(id)}`, { cache: "no-store" })
      const json = await readJson<OrderStatusResponse>(response)
      setOrderStatus(json)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setStatusLoading(false)
    }
  }

  async function createOrder() {
    if (!quote || quoteExpired || !recipient.trim()) return
    setLoading(true)
    setError(null)
    setInfo(null)
    try {
      const response = await fetch("/api/exchange/v1/order", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ quote_id: quote.id, destination: recipient.trim(), refund_address: refund.trim() || undefined }),
      })
      const json = await readJson<OrderResponse>(response)
      setOrder(json)
      setInfo(`Order ${json.id} created. Send ${json.settlement_amount} ${json.settlement_asset} to deposit address.`)
      onExchangeTx({ id: `exchange-${json.id}`, type: "exchange", label: `Exchange ${json.settlement_amount} ${json.settlement_asset} → ${json.pay_amount} ${quote.pay_asset}`, amount: 0, ts: new Date(), status: "pending" })
      await refreshStatus(json.id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  function flipPair() {
    setSource(target)
    setTarget(source)
    setQuote(null)
    setOrder(null)
    setOrderStatus(null)
  }

  async function copy(value: string) {
    await navigator.clipboard.writeText(value).catch(() => {})
    setInfo("Copied to clipboard")
  }

  return (
    <FinanceCard className="min-h-[640px] overflow-hidden p-0">
      <div className="grid h-full grid-cols-[minmax(420px,1fr)_300px]">
        <div className="flex items-start justify-center overflow-y-auto p-5">
          <div className="w-full max-w-xl space-y-3">
            <div className="flex items-center justify-between px-1">
              <div>
                <h2 className="text-lg font-semibold">Exchange</h2>
                <p className="text-xs text-muted-foreground">Provider quote, order creation, and event-derived status.</p>
              </div>
              <div className="flex rounded-full bg-secondary p-1 text-xs">
                {(["instant", "floating"] as const).map((m) => (
                  <button key={m} onClick={() => setMode(m)} className={cn("rounded-full px-3 py-1 font-medium capitalize", mode === m ? "bg-primary text-primary-foreground" : "text-muted-foreground")}>{m}</button>
                ))}
              </div>
            </div>

            <div className="rounded-3xl border border-border bg-background/40 p-3 shadow-lg">
              <SwapAmountPanel label="You pay" value={amount} onChange={setAmount} assetValue={source} assets={assets} onAssetChange={(v) => { setSource(v); setQuote(null) }} />
              <div className="relative flex justify-center">
                <button onClick={flipPair} className="absolute -top-3 z-10 flex h-10 w-10 items-center justify-center rounded-full border border-border bg-card text-muted-foreground shadow-md transition hover:text-foreground" aria-label="Flip exchange pair">
                  <ArrowRightLeft className="h-4 w-4 rotate-90" />
                </button>
              </div>
              <div className="mt-2">
                <SwapAmountPanel label="You receive" value={quote?.pay_amount ?? ""} readOnly assetValue={target} assets={assets} onAssetChange={(v) => { setTarget(v); setQuote(null) }} />
              </div>

              <div className="mt-3 grid grid-cols-2 gap-2 rounded-2xl bg-background/70 p-3">
                <label className="space-y-1">
                  <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Amount side</span>
                  <select value={amountSide} onChange={(e) => setAmountSide(e.target.value as "settlement" | "pay")} className="h-9 w-full rounded-lg bg-secondary px-3 text-sm text-foreground outline-none">
                    <option value="settlement">Exact pay amount</option>
                    <option value="pay">Exact receive amount</option>
                  </select>
                </label>
                <div className="space-y-1">
                  <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Quote</span>
                  <div className="flex h-9 items-center rounded-lg bg-secondary px-3 text-xs text-muted-foreground">{quote ? (quoteExpired ? "Expired" : `${quoteSecondsLeft}s remaining`) : "No quote yet"}</div>
                </div>
              </div>

              <div className="mt-3 space-y-2 rounded-2xl bg-background/70 p-3">
                <input value={recipient} onChange={(e) => setRecipient(e.target.value)} className="h-10 w-full rounded-xl bg-secondary px-3 font-mono text-xs text-foreground outline-none ring-1 ring-transparent focus:ring-primary/50" placeholder={`${selectedTarget.symbol} payout address`} />
                <input value={refund} onChange={(e) => setRefund(e.target.value)} className="h-10 w-full rounded-xl bg-secondary px-3 font-mono text-xs text-foreground outline-none ring-1 ring-transparent focus:ring-primary/50" placeholder={`${selectedSource.symbol} refund address optional`} />
              </div>

              <div className="mt-3 grid grid-cols-2 gap-2">
                <button onClick={requestQuote} disabled={loading || !amount.trim()} className="flex h-11 items-center justify-center gap-2 rounded-2xl bg-primary text-sm font-semibold text-primary-foreground transition-opacity disabled:opacity-40">
                  {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />} Quote
                </button>
                <button onClick={createOrder} disabled={loading || !quote || quoteExpired || !recipient.trim()} className="flex h-11 items-center justify-center gap-2 rounded-2xl bg-secondary text-sm font-semibold transition hover:bg-secondary/80 disabled:opacity-40">
                  <ExternalLink className="h-4 w-4" /> Order
                </button>
              </div>
            </div>

            {error && <div className="rounded-xl border border-[var(--status-error)]/20 bg-[var(--status-error)]/10 p-3 text-xs text-[var(--status-error)]">{error}</div>}
            {info && <div className="rounded-xl border border-primary/20 bg-primary/10 p-3 text-xs text-primary">{info}</div>}
            {quote && (
              <div className="grid grid-cols-4 gap-2 rounded-2xl border border-border bg-background/60 p-3 text-xs">
                <div><p className="text-muted-foreground">Pay</p><p className="font-mono">{quote.settlement_amount}</p></div>
                <div><p className="text-muted-foreground">Receive</p><p className="font-mono">{quote.pay_amount}</p></div>
                <div><p className="text-muted-foreground">Rate</p><p className="font-mono">{quote.rate}</p></div>
                <div><p className="text-muted-foreground">ETA</p><p className="font-mono">{quote.estimated_seconds ? `${Math.round(quote.estimated_seconds / 60)}m` : "unknown"}</p></div>
              </div>
            )}
          </div>
        </div>

        <aside className="border-l border-border bg-[var(--window-header)]/30 p-4">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-sm font-semibold">Order</h3>
            {order && <button onClick={() => refreshStatus()} disabled={statusLoading} className="rounded p-1 text-muted-foreground hover:bg-secondary hover:text-foreground"><RefreshCw className={cn("h-3.5 w-3.5", statusLoading && "animate-spin")} /></button>}
          </div>
          {!order ? <div className="rounded-lg border border-dashed border-border p-4 text-xs text-muted-foreground">Create an order from a live quote. The backend returns a deposit address and derives status from exchange events.</div> : (
            <div className="space-y-3">
              <div className={cn("rounded-lg p-2 text-xs font-medium", statusTone(orderStatus?.status || order.status))}>{orderStatus?.status || order.status}</div>
              <div className="space-y-1"><p className="text-[10px] uppercase tracking-wider text-muted-foreground">Deposit address</p><button onClick={() => copy(order.deposit_address)} className="w-full rounded-lg bg-background p-2 text-left font-mono text-[11px] ring-1 ring-border hover:ring-primary/40">{order.deposit_address}</button></div>
              <div className="grid grid-cols-2 gap-2 text-xs"><div className="rounded-lg bg-background p-2 ring-1 ring-border"><p className="text-muted-foreground">Send</p><p className="font-mono">{order.settlement_amount}</p></div><div className="rounded-lg bg-background p-2 ring-1 ring-border"><p className="text-muted-foreground">Receive</p><p className="font-mono">{order.pay_amount}</p></div></div>
              {orderStatus && <div className="space-y-1 rounded-lg bg-background p-2 text-[11px] ring-1 ring-border"><div className="flex justify-between"><span className="text-muted-foreground">Events</span><span>{orderStatus.event_count ?? 0}</span></div><div className="flex justify-between"><span className="text-muted-foreground">Terminal</span><span>{orderStatus.terminal ? "yes" : "no"}</span></div><div className="flex justify-between"><span className="text-muted-foreground">Manual review</span><span>{orderStatus.manual_review_required ? "yes" : "no"}</span></div>{orderStatus.last_event_type && <div className="flex justify-between gap-2"><span className="text-muted-foreground">Last event</span><span className="truncate">{orderStatus.last_event_type}</span></div>}</div>}
            </div>
          )}
        </aside>
      </div>
    </FinanceCard>
  )
}

function QuickSend({ balance, onSend }: { balance: number; onSend: (tx: Transaction, amount: number) => void }) {
  const [to, setTo] = React.useState("")
  const [amount, setAmount] = React.useState("")
  const [note, setNote] = React.useState("")
  const [sending, setSending] = React.useState(false)
  const [done, setDone] = React.useState(false)

  function submit() {
    const amt = parseFloat(amount)
    if (!to.trim() || isNaN(amt) || amt <= 0 || amt > balance) return
    setSending(true)
    setTimeout(() => {
      onSend({ id: `t-${Date.now()}`, type: "send", label: note.trim() || `Sent to ${to.trim()}`, amount: -amt, ts: new Date(), status: "confirmed" }, amt)
      setSending(false)
      setDone(true)
      setTo("")
      setAmount("")
      setNote("")
      setTimeout(() => setDone(false), 1500)
    }, 900)
  }

  return (
    <FinanceCard>
      <h3 className="mb-3 text-sm font-semibold">Quick send</h3>
      {done ? <div className="flex flex-col items-center gap-2 py-6 text-center"><div className="flex h-12 w-12 items-center justify-center rounded-full bg-[var(--status-online)]/15"><Check className="h-6 w-6 text-[var(--status-online)]" /></div><p className="text-sm font-medium">Transfer confirmed</p></div> : (
        <div className="space-y-2">
          <input value={to} onChange={(e) => setTo(e.target.value)} placeholder="@handle or edge1q..." className="h-10 w-full rounded-lg bg-secondary px-3 text-sm outline-none focus:ring-1 focus:ring-primary" />
          <input type="number" min="0.01" step="0.01" value={amount} onChange={(e) => setAmount(e.target.value)} placeholder="0.00 EDGE" className="h-10 w-full rounded-lg bg-secondary px-3 font-mono text-sm outline-none focus:ring-1 focus:ring-primary" />
          <input value={note} onChange={(e) => setNote(e.target.value)} placeholder="Note" className="h-10 w-full rounded-lg bg-secondary px-3 text-sm outline-none focus:ring-1 focus:ring-primary" />
          <button onClick={submit} disabled={sending || !to.trim() || !amount || parseFloat(amount) <= 0 || parseFloat(amount) > balance} className="flex h-10 w-full items-center justify-center gap-2 rounded-lg bg-primary text-sm font-medium text-primary-foreground disabled:opacity-40">{sending ? <RefreshCw className="h-4 w-4 animate-spin" /> : <><Send className="h-4 w-4" />Send</>}</button>
        </div>
      )}
    </FinanceCard>
  )
}

function ReceiveCard({ copied, onCopy }: { copied: boolean; onCopy: () => void }) {
  return (
    <FinanceCard>
      <h3 className="mb-3 text-sm font-semibold">Receive</h3>
      <div className="flex items-center gap-3">
        <div className="relative flex h-20 w-20 items-center justify-center rounded-xl border border-border bg-secondary/50">
          <div className="grid grid-cols-5 gap-0.5 opacity-60">{Array.from({ length: 25 }).map((_, i) => <div key={i} className="h-2.5 w-2.5 rounded-sm" style={{ background: Math.sin(i * 7.3 + 1.1) > 0.1 ? "oklch(0.65 0.2 145)" : "transparent" }} />)}</div>
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-[10px] text-muted-foreground">Your EDGE address</p>
          <p className="break-all font-mono text-[11px]">{ADDRESS}</p>
          <button onClick={onCopy} className="mt-2 flex items-center gap-1 rounded bg-secondary px-2 py-1 text-[11px]">{copied ? <Check className="h-3 w-3 text-[var(--status-online)]" /> : <Copy className="h-3 w-3" />}Copy</button>
        </div>
      </div>
    </FinanceCard>
  )
}

export function FinancesApp() {
  const [balance, setBalance] = React.useState(134.57)
  const [txs, setTxs] = React.useState<Transaction[]>(DEMO_TXS)
  const [copied, setCopied] = React.useState(false)
  const totalValue = balance * 1.84

  function copyAddress() {
    navigator.clipboard.writeText(ADDRESS).catch(() => {})
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  function addTx(tx: Transaction) {
    setTxs((prev) => [tx, ...prev])
  }

  return (
    <div className="h-full overflow-auto bg-background p-4 text-foreground">
      <div className="grid min-h-full grid-cols-[minmax(320px,1.1fr)_minmax(420px,1.5fr)_320px] gap-4">
        <div className="space-y-4">
          <FinanceCard className="relative overflow-hidden bg-gradient-to-br from-[oklch(0.14_0.02_145)] to-[oklch(0.1_0.005_280)]">
            <div className="absolute right-4 top-4 opacity-5"><Shield className="h-24 w-24" /></div>
            <p className="text-[10px] font-medium uppercase tracking-widest text-muted-foreground">Total balance</p>
            <p className="mt-1 text-4xl font-bold tabular-nums">{balance.toFixed(2)} <span className="text-base font-normal text-muted-foreground">EDGE</span></p>
            <p className="mt-1 text-sm text-muted-foreground">≈ ${totalValue.toFixed(2)} USD</p>
            <div className="mt-4 flex items-center gap-1.5"><p className="font-mono text-[11px] text-muted-foreground">{SHORT_ADDRESS}</p><button onClick={copyAddress} className="rounded p-0.5 text-muted-foreground transition-colors hover:text-foreground">{copied ? <Check className="h-3.5 w-3.5 text-[var(--status-online)]" /> : <Copy className="h-3.5 w-3.5" />}</button></div>
          </FinanceCard>
          <AllocationCard totalValue={totalValue} />
          <FinanceCard>
            <h3 className="mb-3 text-sm font-semibold">Assets</h3>
            <div className="space-y-2">{portfolioAssets.map((asset) => <div key={asset.symbol} className="flex items-center gap-2 rounded-lg bg-background p-2 ring-1 ring-border"><AssetIcon asset={asset} /><div className="min-w-0 flex-1"><p className="text-xs font-semibold">{asset.symbol}</p><p className="text-[10px] text-muted-foreground">{asset.network}</p></div><div className="text-right"><p className="font-mono text-xs">{asset.amount}</p><p className="text-[10px] text-muted-foreground">${asset.value.toFixed(2)}</p></div></div>)}</div>
          </FinanceCard>
        </div>

        <ExchangeWorkspace onExchangeTx={addTx} />

        <div className="space-y-4">
          <QuickSend balance={balance} onSend={(tx, amount) => { addTx(tx); setBalance((b) => parseFloat((b - amount).toFixed(2))) }} />
          <ReceiveCard copied={copied} onCopy={copyAddress} />
          <FinanceCard>
            <h3 className="mb-3 text-sm font-semibold">Recent activity</h3>
            <div className="max-h-[280px] divide-y divide-border overflow-y-auto">{txs.map((tx) => <div key={tx.id} className="flex items-center gap-3 py-3"><TxIcon type={tx.type} /><div className="min-w-0 flex-1"><p className="truncate text-sm font-medium">{tx.label}</p><p className="text-[10px] text-muted-foreground">{formatDate(tx.ts)}</p></div><div className="text-right"><p className={cn("text-xs font-semibold tabular-nums", tx.amount > 0 ? "text-[var(--status-online)]" : "text-foreground")}>{tx.amount > 0 ? "+" : ""}{tx.amount.toFixed(2)} EDGE</p><p className="text-[10px] text-muted-foreground capitalize">{tx.status}</p></div></div>)}</div>
          </FinanceCard>
        </div>
      </div>
    </div>
  )
}

export function WalletApp() {
  return <FinancesApp />
}
