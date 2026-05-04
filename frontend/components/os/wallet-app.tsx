"use client"

import { useEffect, useMemo, useState } from "react"
import {
  ArrowDownLeft,
  ArrowRightLeft,
  ArrowUpRight,
  Check,
  Copy,
  ExternalLink,
  Plus,
  RefreshCw,
  Send,
  Shield,
  WalletCards,
} from "lucide-react"
import { cn } from "@/lib/utils"

type TxType = "send" | "receive" | "earn" | "exchange"
type Tab = "balance" | "exchange" | "send" | "receive"

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
  latest_provider_status_detail?: string
  last_event_type?: string
  updated_at_ms?: number
}

interface Transaction {
  id: string
  type: TxType
  label: string
  amount: number
  from?: string
  to?: string
  ts: Date
  status: "confirmed" | "pending"
}

const DEMO_TXS: Transaction[] = [
  { id: "t1", type: "receive", label: "Compute reward", amount: 4.82, from: "Edgerun Protocol", ts: new Date(Date.now() - 3600000), status: "confirmed" },
  { id: "t2", type: "send", label: "Paid Elias Voss", amount: -2.00, to: "@voss.edge", ts: new Date(Date.now() - 7200000), status: "confirmed" },
  { id: "t3", type: "earn", label: "Node uptime bonus", amount: 1.20, from: "Edgerun Protocol", ts: new Date(Date.now() - 86400000), status: "confirmed" },
  { id: "t4", type: "receive", label: "Transfer from Ara", amount: 10.00, from: "@ara.run", ts: new Date(Date.now() - 172800000), status: "confirmed" },
]

const ADDRESS = "edge1qxy2kgdygjrsqtzq2n0yrf249.run"
const SHORT_ADDRESS = "edge1qxy…249.run"
const FALLBACK_ASSETS: AssetInfo[] = [
  { symbol: "USDT", network: "Tron", contract: "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t" },
  { symbol: "BTC", network: "Bitcoin" },
  { symbol: "ETH", network: "Ethereum" },
  { symbol: "DOGE", network: "Dogecoin" },
  { symbol: "SOL", network: "Solana" },
]

function TxIcon({ type }: { type: TxType }) {
  const config = {
    send: { icon: <ArrowUpRight className="h-4 w-4" />, color: "text-[var(--status-error)] bg-[var(--status-error)]/10" },
    receive: { icon: <ArrowDownLeft className="h-4 w-4" />, color: "text-[var(--status-online)] bg-[var(--status-online)]/10" },
    earn: { icon: <RefreshCw className="h-4 w-4" />, color: "text-[var(--status-warning)] bg-[var(--status-warning)]/10" },
    exchange: { icon: <ArrowRightLeft className="h-4 w-4" />, color: "text-primary bg-primary/10" },
  }[type]
  return <div className={cn("flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full", config.color)}>{config.icon}</div>
}

function formatDate(d: Date) {
  const now = Date.now()
  const diff = now - d.getTime()
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" })
}

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
  if (!response.ok || json?.error) {
    throw new Error(json?.error || json?.detail || `HTTP ${response.status}`)
  }
  return json as T
}

function statusTone(status: string) {
  const normalized = status.toLowerCase()
  if (["completed", "refunded"].includes(normalized)) return "text-[var(--status-online)] bg-[var(--status-online)]/10"
  if (["failed", "rejected", "canceled", "expired", "refund_required"].includes(normalized)) return "text-[var(--status-error)] bg-[var(--status-error)]/10"
  if (["action_required", "on_hold", "manual_review_required"].includes(normalized)) return "text-[var(--status-warning)] bg-[var(--status-warning)]/10"
  return "text-primary bg-primary/10"
}

function ExchangePanel({ onExchangeTx }: { onExchangeTx: (tx: Transaction) => void }) {
  const [assets, setAssets] = useState<AssetInfo[]>(FALLBACK_ASSETS)
  const [source, setSource] = useState("USDT:Tron")
  const [target, setTarget] = useState("BTC:Bitcoin")
  const [amount, setAmount] = useState("100")
  const [amountSide, setAmountSide] = useState<"settlement" | "pay">("settlement")
  const [mode, setMode] = useState<"instant" | "floating">("instant")
  const [recipient, setRecipient] = useState("")
  const [refund, setRefund] = useState("")
  const [quote, setQuote] = useState<QuoteResponse | null>(null)
  const [order, setOrder] = useState<OrderResponse | null>(null)
  const [orderStatus, setOrderStatus] = useState<OrderStatusResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [statusLoading, setStatusLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [info, setInfo] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    async function loadAssets() {
      try {
        const response = await fetch("/api/exchange/v1/assets", { cache: "no-store" })
        const json = await readJson<{ assets: Record<string, { network: string; contract?: string }> }>(response)
        const parsed = Object.entries(json.assets).map(([symbol, value]) => ({ symbol, network: value.network, contract: value.contract }))
        if (!cancelled && parsed.length > 0) setAssets(parsed)
      } catch (err) {
        if (!cancelled) setInfo(`Exchange API unavailable; showing static asset catalog. ${err instanceof Error ? err.message : String(err)}`)
      }
    }
    loadAssets()
    return () => { cancelled = true }
  }, [])

  const selectedSource = useMemo(() => parseAssetId(source), [source])
  const selectedTarget = useMemo(() => parseAssetId(target), [target])
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
      onExchangeTx({
        id: `exchange-${json.id}`,
        type: "exchange",
        label: `Exchange ${json.settlement_amount} ${json.settlement_asset} → ${json.pay_amount} ${quote.pay_asset}`,
        amount: 0,
        ts: new Date(),
        status: "pending",
      })
      await refreshStatus(json.id)
    } catch (err) {
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

  async function copy(value: string) {
    await navigator.clipboard.writeText(value).catch(() => {})
    setInfo("Copied to clipboard")
  }

  return (
    <div className="grid h-full grid-cols-[1fr_280px] overflow-hidden">
      <div className="space-y-4 overflow-y-auto p-5">
        <div className="rounded-xl border border-border bg-secondary/30 p-4">
          <div className="mb-4 flex items-center justify-between">
            <div>
              <h3 className="flex items-center gap-2 text-sm font-semibold text-foreground"><ArrowRightLeft className="h-4 w-4 text-primary" /> Exchange</h3>
              <p className="mt-0.5 text-xs text-muted-foreground">Provider-routed quote → stored quote → provider order → event-derived status.</p>
            </div>
            <span className="rounded-md bg-primary/10 px-2 py-1 text-[10px] font-medium text-primary">/v1 quote/order</span>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <label className="space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">You send</span>
              <select value={source} onChange={(e) => setSource(e.target.value)} className="h-9 w-full rounded-lg bg-background px-3 text-sm text-foreground outline-none ring-1 ring-border focus:ring-primary/50">
                {assets.map((asset) => <option key={assetId(asset)} value={assetId(asset)}>{asset.symbol} · {asset.network}</option>)}
              </select>
            </label>
            <label className="space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">You receive</span>
              <select value={target} onChange={(e) => setTarget(e.target.value)} className="h-9 w-full rounded-lg bg-background px-3 text-sm text-foreground outline-none ring-1 ring-border focus:ring-primary/50">
                {assets.map((asset) => <option key={assetId(asset)} value={assetId(asset)}>{asset.symbol} · {asset.network}</option>)}
              </select>
            </label>
          </div>

          <div className="mt-3 grid grid-cols-[1fr_120px_120px] gap-3">
            <label className="space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Amount</span>
              <input value={amount} onChange={(e) => setAmount(e.target.value)} className="h-9 w-full rounded-lg bg-background px-3 font-mono text-sm text-foreground outline-none ring-1 ring-border focus:ring-primary/50" placeholder="100" />
            </label>
            <label className="space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Side</span>
              <select value={amountSide} onChange={(e) => setAmountSide(e.target.value as "settlement" | "pay")} className="h-9 w-full rounded-lg bg-background px-3 text-sm text-foreground outline-none ring-1 ring-border focus:ring-primary/50">
                <option value="settlement">Send</option>
                <option value="pay">Receive</option>
              </select>
            </label>
            <label className="space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Mode</span>
              <select value={mode} onChange={(e) => setMode(e.target.value as "instant" | "floating")} className="h-9 w-full rounded-lg bg-background px-3 text-sm text-foreground outline-none ring-1 ring-border focus:ring-primary/50">
                <option value="instant">Instant</option>
                <option value="floating">Floating</option>
              </select>
            </label>
          </div>

          <div className="mt-3 space-y-3">
            <label className="block space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Recipient address for payout</span>
              <input value={recipient} onChange={(e) => setRecipient(e.target.value)} className="h-9 w-full rounded-lg bg-background px-3 font-mono text-xs text-foreground outline-none ring-1 ring-border focus:ring-primary/50" placeholder={`${selectedTarget.symbol} ${selectedTarget.network} address`} />
            </label>
            <label className="block space-y-1">
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Refund address optional</span>
              <input value={refund} onChange={(e) => setRefund(e.target.value)} className="h-9 w-full rounded-lg bg-background px-3 font-mono text-xs text-foreground outline-none ring-1 ring-border focus:ring-primary/50" placeholder={`${selectedSource.symbol} ${selectedSource.network} refund address`} />
            </label>
          </div>

          <div className="mt-4 grid grid-cols-2 gap-2">
            <button onClick={requestQuote} disabled={loading || !amount.trim()} className="flex h-10 items-center justify-center gap-2 rounded-lg bg-primary text-sm font-medium text-primary-foreground transition-opacity disabled:opacity-40">
              {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />} Get quote
            </button>
            <button onClick={createOrder} disabled={loading || !quote || quoteExpired || !recipient.trim()} className="flex h-10 items-center justify-center gap-2 rounded-lg bg-secondary text-sm font-medium text-foreground transition-opacity hover:bg-secondary/80 disabled:opacity-40">
              <ExternalLink className="h-4 w-4" /> Create order
            </button>
          </div>
        </div>

        {error && <div className="rounded-lg border border-[var(--status-error)]/20 bg-[var(--status-error)]/10 p-3 text-xs text-[var(--status-error)]">{error}</div>}
        {info && <div className="rounded-lg border border-primary/20 bg-primary/10 p-3 text-xs text-primary">{info}</div>}

        {quote && (
          <div className="rounded-xl border border-border bg-secondary/30 p-4">
            <div className="mb-3 flex items-center justify-between">
              <h3 className="text-sm font-semibold text-foreground">Quote</h3>
              <span className={cn("rounded px-2 py-1 text-[10px] font-medium", quoteExpired ? "bg-[var(--status-error)]/10 text-[var(--status-error)]" : "bg-[var(--status-online)]/10 text-[var(--status-online)]")}>{quoteExpired ? "Expired" : `${quoteSecondsLeft}s left`}</span>
            </div>
            <div className="grid grid-cols-2 gap-3 text-xs">
              <div><p className="text-muted-foreground">Send</p><p className="font-mono text-foreground">{quote.settlement_amount} {quote.settlement_asset}</p></div>
              <div><p className="text-muted-foreground">Receive</p><p className="font-mono text-foreground">{quote.pay_amount} {quote.pay_asset}</p></div>
              <div><p className="text-muted-foreground">Rate</p><p className="font-mono text-foreground">{quote.rate}</p></div>
              <div><p className="text-muted-foreground">ETA</p><p className="font-mono text-foreground">{quote.estimated_seconds ? `${Math.round(quote.estimated_seconds / 60)} min` : "unknown"}</p></div>
            </div>
          </div>
        )}
      </div>

      <aside className="border-l border-border bg-[var(--window-header)]/30 p-4">
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-foreground">Order</h3>
          {order && <button onClick={() => refreshStatus()} disabled={statusLoading} className="rounded p-1 text-muted-foreground hover:bg-secondary hover:text-foreground"><RefreshCw className={cn("h-3.5 w-3.5", statusLoading && "animate-spin")} /></button>}
        </div>

        {!order ? (
          <div className="rounded-lg border border-dashed border-border p-4 text-xs text-muted-foreground">Create an order from a non-expired quote. Backend will return the deposit address and store provider state for event-derived status projection.</div>
        ) : (
          <div className="space-y-3">
            <div className={cn("rounded-lg p-2 text-xs font-medium", statusTone(orderStatus?.status || order.status))}>{orderStatus?.status || order.status}</div>
            <div className="space-y-1">
              <p className="text-[10px] uppercase tracking-wider text-muted-foreground">Deposit address</p>
              <button onClick={() => copy(order.deposit_address)} className="w-full rounded-lg bg-background p-2 text-left font-mono text-[11px] text-foreground ring-1 ring-border hover:ring-primary/40">{order.deposit_address}</button>
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="rounded-lg bg-background p-2 ring-1 ring-border"><p className="text-muted-foreground">Send</p><p className="font-mono text-foreground">{order.settlement_amount}</p></div>
              <div className="rounded-lg bg-background p-2 ring-1 ring-border"><p className="text-muted-foreground">Receive</p><p className="font-mono text-foreground">{order.pay_amount}</p></div>
            </div>
            {orderStatus && (
              <div className="space-y-1 rounded-lg bg-background p-2 text-[11px] ring-1 ring-border">
                <div className="flex justify-between"><span className="text-muted-foreground">Events</span><span>{orderStatus.event_count ?? 0}</span></div>
                <div className="flex justify-between"><span className="text-muted-foreground">Terminal</span><span>{orderStatus.terminal ? "yes" : "no"}</span></div>
                <div className="flex justify-between"><span className="text-muted-foreground">Manual review</span><span>{orderStatus.manual_review_required ? "yes" : "no"}</span></div>
                {orderStatus.last_event_type && <div className="flex justify-between gap-2"><span className="text-muted-foreground">Last event</span><span className="truncate">{orderStatus.last_event_type}</span></div>}
              </div>
            )}
          </div>
        )}
      </aside>
    </div>
  )
}

export function WalletApp() {
  const [balance, setBalance] = useState(134.57)
  const [txs, setTxs] = useState<Transaction[]>(DEMO_TXS)
  const [tab, setTab] = useState<Tab>("balance")
  const [copied, setCopied] = useState(false)
  const [sendTo, setSendTo] = useState("")
  const [sendAmount, setSendAmount] = useState("")
  const [sendNote, setSendNote] = useState("")
  const [sending, setSending] = useState(false)
  const [sendDone, setSendDone] = useState(false)

  const handleCopy = () => {
    navigator.clipboard.writeText(ADDRESS).catch(() => {})
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleSend = () => {
    const amt = parseFloat(sendAmount)
    if (!sendTo.trim() || isNaN(amt) || amt <= 0 || amt > balance) return
    setSending(true)
    setTimeout(() => {
      const tx: Transaction = { id: `t-${Date.now()}`, type: "send", label: sendNote.trim() || `Sent to ${sendTo.trim()}`, amount: -amt, to: sendTo.trim(), ts: new Date(), status: "confirmed" }
      setTxs((prev) => [tx, ...prev])
      setBalance((b) => parseFloat((b - amt).toFixed(2)))
      setSending(false)
      setSendDone(true)
      setSendTo("")
      setSendAmount("")
      setSendNote("")
      setTimeout(() => { setSendDone(false); setTab("balance") }, 1500)
    }, 1200)
  }

  const tabs: { id: Tab; label: string; icon: React.ReactNode }[] = [
    { id: "balance", label: "Balance", icon: <WalletCards className="h-3.5 w-3.5" /> },
    { id: "exchange", label: "Exchange", icon: <ArrowRightLeft className="h-3.5 w-3.5" /> },
    { id: "send", label: "Send", icon: <Send className="h-3.5 w-3.5" /> },
    { id: "receive", label: "Receive", icon: <ArrowDownLeft className="h-3.5 w-3.5" /> },
  ]

  return (
    <div className="flex h-full flex-col">
      <div className="relative overflow-hidden border-b border-[var(--window-border)] bg-gradient-to-br from-[oklch(0.14_0.02_145)] to-[oklch(0.1_0.005_280)] px-6 py-5">
        <div className="absolute right-4 top-4 opacity-5"><Shield className="h-24 w-24" /></div>
        <p className="text-[10px] font-medium uppercase tracking-widest text-muted-foreground">EDGE Balance</p>
        <p className="mt-1 text-3xl font-bold tabular-nums text-foreground">{balance.toFixed(2)} <span className="text-lg font-normal text-muted-foreground">EDGE</span></p>
        <p className="mt-0.5 text-xs text-muted-foreground">≈ ${(balance * 1.84).toFixed(2)} USD</p>
        <div className="mt-4 flex items-center gap-1.5"><p className="font-mono text-[11px] text-muted-foreground">{SHORT_ADDRESS}</p><button onClick={handleCopy} className="rounded p-0.5 text-muted-foreground transition-colors hover:text-foreground">{copied ? <Check className="h-3.5 w-3.5 text-[var(--status-online)]" /> : <Copy className="h-3.5 w-3.5" />}</button></div>
      </div>

      <div className="flex border-b border-[var(--window-border)]">
        {tabs.map((t) => <button key={t.id} onClick={() => setTab(t.id)} className={cn("flex flex-1 items-center justify-center gap-1.5 py-2.5 text-xs font-medium transition-colors", tab === t.id ? "border-b-2 border-primary text-primary" : "text-muted-foreground hover:text-foreground")}>{t.icon}{t.label}</button>)}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {tab === "exchange" && <ExchangePanel onExchangeTx={(tx) => setTxs((prev) => [tx, ...prev])} />}
        {tab === "balance" && <div className="divide-y divide-[var(--window-border)]">{txs.map((tx) => <div key={tx.id} className="flex items-center gap-3 px-4 py-3"><TxIcon type={tx.type} /><div className="min-w-0 flex-1"><p className="truncate text-sm font-medium text-foreground">{tx.label}</p><p className="text-[10px] text-muted-foreground">{formatDate(tx.ts)}</p></div><div className="text-right"><p className={cn("text-sm font-semibold tabular-nums", tx.amount > 0 ? "text-[var(--status-online)]" : "text-foreground")}>{tx.amount > 0 ? "+" : ""}{tx.amount.toFixed(2)} EDGE</p><p className="text-[10px] text-muted-foreground capitalize">{tx.status}</p></div></div>)}</div>}
        {tab === "send" && <div className="space-y-4 p-5">{sendDone ? <div className="flex flex-col items-center gap-3 py-8 text-center"><div className="flex h-14 w-14 items-center justify-center rounded-full bg-[var(--status-online)]/15"><Check className="h-7 w-7 text-[var(--status-online)]" /></div><p className="text-sm font-medium text-foreground">Transfer confirmed</p><p className="text-xs text-muted-foreground">Settled on the Edgerun ledger</p></div> : <><label className="block space-y-1"><span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Recipient handle or address</span><input value={sendTo} onChange={(e) => setSendTo(e.target.value)} placeholder="@handle or edge1q..." className="h-9 w-full rounded-lg bg-secondary px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary" /></label><label className="block space-y-1"><span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Amount (EDGE)</span><input type="number" min="0.01" step="0.01" value={sendAmount} onChange={(e) => setSendAmount(e.target.value)} placeholder="0.00" className="h-9 w-full rounded-lg bg-secondary px-3 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary" /><p className="text-[10px] text-muted-foreground">Available: {balance.toFixed(2)} EDGE</p></label><label className="block space-y-1"><span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Note (optional)</span><input value={sendNote} onChange={(e) => setSendNote(e.target.value)} placeholder="What's this for?" className="h-9 w-full rounded-lg bg-secondary px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary" /></label><button onClick={handleSend} disabled={sending || !sendTo.trim() || !sendAmount || parseFloat(sendAmount) <= 0 || parseFloat(sendAmount) > balance} className="flex h-10 w-full items-center justify-center gap-2 rounded-lg bg-primary text-sm font-medium text-primary-foreground transition-opacity disabled:opacity-40">{sending ? <RefreshCw className="h-4 w-4 animate-spin" /> : <><Send className="h-4 w-4" />Send</>}</button></>}</div>}
        {tab === "receive" && <div className="flex flex-col items-center gap-5 p-6"><div className="relative flex h-40 w-40 items-center justify-center rounded-xl border border-[var(--window-border)] bg-secondary/50"><div className="grid grid-cols-7 gap-0.5 p-2 opacity-60">{Array.from({ length: 49 }).map((_, i) => <div key={i} className="h-4 w-4 rounded-sm" style={{ background: (Math.sin(i * 7.3 + 1.1) > 0.1) ? "oklch(0.65 0.2 145)" : "transparent" }} />)}</div><div className="absolute inset-0 flex items-center justify-center"><div className="flex h-8 w-8 items-center justify-center rounded-md bg-[var(--window-bg)] text-[10px] font-bold text-primary">ER</div></div></div><div className="text-center"><p className="mb-1 text-[10px] text-muted-foreground">Your EDGE address</p><p className="break-all font-mono text-xs text-foreground">{ADDRESS}</p></div><button onClick={handleCopy} className="flex items-center gap-2 rounded-lg bg-secondary px-4 py-2 text-xs font-medium text-foreground transition-colors hover:bg-secondary/70">{copied ? <Check className="h-3.5 w-3.5 text-[var(--status-online)]" /> : <Copy className="h-3.5 w-3.5" />}{copied ? "Copied!" : "Copy address"}</button><div className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60"><Shield className="h-3 w-3" /><span>Identity-bound to your fingerprint key</span></div></div>}
      </div>
    </div>
  )
}
