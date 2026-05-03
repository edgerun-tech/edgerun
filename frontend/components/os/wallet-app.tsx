"use client"

import { useState } from "react"
import { ArrowUpRight, ArrowDownLeft, Copy, Check, RefreshCw, Send, Plus, Shield } from "lucide-react"
import { cn } from "@/lib/utils"

type TxType = "send" | "receive" | "earn"

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
  { id: "t5", type: "send", label: "App Store – Compute", amount: -10.00, to: "App Store", ts: new Date(Date.now() - 259200000), status: "confirmed" },
  { id: "t6", type: "earn", label: "Bandwidth credit", amount: 0.55, from: "Edgerun Protocol", ts: new Date(Date.now() - 345600000), status: "confirmed" },
]

const ADDRESS = "edge1qxy2kgdygjrsqtzq2n0yrf249.run"
const SHORT_ADDRESS = "edge1qxy…249.run"

function TxIcon({ type }: { type: TxType }) {
  const config = {
    send: { icon: <ArrowUpRight className="h-4 w-4" />, color: "text-[var(--status-error)] bg-[var(--status-error)]/10" },
    receive: { icon: <ArrowDownLeft className="h-4 w-4" />, color: "text-[var(--status-online)] bg-[var(--status-online)]/10" },
    earn: { icon: <RefreshCw className="h-4 w-4" />, color: "text-[var(--status-warning)] bg-[var(--status-warning)]/10" },
  }[type]
  return (
    <div className={cn("flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full", config.color)}>
      {config.icon}
    </div>
  )
}

function formatDate(d: Date) {
  const now = Date.now()
  const diff = now - d.getTime()
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" })
}

type Tab = "balance" | "send" | "receive"

export function WalletApp() {
  const [balance, setBalance] = useState(134.57)
  const [txs, setTxs] = useState<Transaction[]>(DEMO_TXS)
  const [tab, setTab] = useState<Tab>("balance")
  const [copied, setCopied] = useState(false)

  // Send form
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
      const tx: Transaction = {
        id: `t-${Date.now()}`,
        type: "send",
        label: sendNote.trim() || `Sent to ${sendTo.trim()}`,
        amount: -amt,
        to: sendTo.trim(),
        ts: new Date(),
        status: "confirmed",
      }
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

  const tabs: { id: Tab; label: string }[] = [
    { id: "balance", label: "Balance" },
    { id: "send", label: "Send" },
    { id: "receive", label: "Receive" },
  ]

  return (
    <div className="flex h-full flex-col">
      {/* Card */}
      <div className="relative overflow-hidden border-b border-[var(--window-border)] bg-gradient-to-br from-[oklch(0.14_0.02_145)] to-[oklch(0.1_0.005_280)] px-6 py-5">
        <div className="absolute right-4 top-4 opacity-5">
          <Shield className="h-24 w-24" />
        </div>
        <p className="text-[10px] font-medium uppercase tracking-widest text-muted-foreground">EDGE Balance</p>
        <p className="mt-1 text-3xl font-bold tabular-nums text-foreground">{balance.toFixed(2)} <span className="text-lg font-normal text-muted-foreground">EDGE</span></p>
        <p className="mt-0.5 text-xs text-muted-foreground">≈ ${(balance * 1.84).toFixed(2)} USD</p>
        <div className="mt-4 flex items-center gap-1.5">
          <p className="font-mono text-[11px] text-muted-foreground">{SHORT_ADDRESS}</p>
          <button onClick={handleCopy} className="rounded p-0.5 text-muted-foreground transition-colors hover:text-foreground">
            {copied ? <Check className="h-3.5 w-3.5 text-[var(--status-online)]" /> : <Copy className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-[var(--window-border)]">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={cn(
              "flex-1 py-2.5 text-xs font-medium transition-colors",
              tab === t.id
                ? "border-b-2 border-primary text-primary"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {/* Balance / Transactions */}
        {tab === "balance" && (
          <div className="divide-y divide-[var(--window-border)]">
            {txs.length === 0 && (
              <p className="p-6 text-center text-xs text-muted-foreground">No transactions yet</p>
            )}
            {txs.map((tx) => (
              <div key={tx.id} className="flex items-center gap-3 px-4 py-3">
                <TxIcon type={tx.type} />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-foreground">{tx.label}</p>
                  <p className="text-[10px] text-muted-foreground">{formatDate(tx.ts)}</p>
                </div>
                <div className="text-right">
                  <p className={cn(
                    "text-sm font-semibold tabular-nums",
                    tx.amount > 0 ? "text-[var(--status-online)]" : "text-foreground"
                  )}>
                    {tx.amount > 0 ? "+" : ""}{tx.amount.toFixed(2)} EDGE
                  </p>
                  <p className="text-[10px] text-muted-foreground capitalize">{tx.status}</p>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Send */}
        {tab === "send" && (
          <div className="p-5 space-y-4">
            {sendDone ? (
              <div className="flex flex-col items-center gap-3 py-8 text-center">
                <div className="flex h-14 w-14 items-center justify-center rounded-full bg-[var(--status-online)]/15">
                  <Check className="h-7 w-7 text-[var(--status-online)]" />
                </div>
                <p className="text-sm font-medium text-foreground">Transfer confirmed</p>
                <p className="text-xs text-muted-foreground">Settled on the Edgerun ledger</p>
              </div>
            ) : (
              <>
                <div className="space-y-1">
                  <label className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Recipient handle or address</label>
                  <input
                    value={sendTo}
                    onChange={(e) => setSendTo(e.target.value)}
                    placeholder="@handle or edge1q..."
                    className="h-9 w-full rounded-lg bg-secondary px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Amount (EDGE)</label>
                  <input
                    type="number"
                    min="0.01"
                    step="0.01"
                    value={sendAmount}
                    onChange={(e) => setSendAmount(e.target.value)}
                    placeholder="0.00"
                    className="h-9 w-full rounded-lg bg-secondary px-3 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                  />
                  <p className="text-[10px] text-muted-foreground">Available: {balance.toFixed(2)} EDGE</p>
                </div>
                <div className="space-y-1">
                  <label className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Note (optional)</label>
                  <input
                    value={sendNote}
                    onChange={(e) => setSendNote(e.target.value)}
                    placeholder="What's this for?"
                    className="h-9 w-full rounded-lg bg-secondary px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                  />
                </div>
                <button
                  onClick={handleSend}
                  disabled={sending || !sendTo.trim() || !sendAmount || parseFloat(sendAmount) <= 0 || parseFloat(sendAmount) > balance}
                  className="flex h-10 w-full items-center justify-center gap-2 rounded-lg bg-primary font-medium text-sm text-primary-foreground transition-opacity disabled:opacity-40"
                >
                  {sending ? (
                    <RefreshCw className="h-4 w-4 animate-spin" />
                  ) : (
                    <><Send className="h-4 w-4" />Send</>
                  )}
                </button>
              </>
            )}
          </div>
        )}

        {/* Receive */}
        {tab === "receive" && (
          <div className="flex flex-col items-center gap-5 p-6">
            {/* QR-like visual placeholder */}
            <div className="relative flex h-40 w-40 items-center justify-center rounded-xl border border-[var(--window-border)] bg-secondary/50">
              <div className="grid grid-cols-7 gap-0.5 p-2 opacity-60">
                {Array.from({ length: 49 }).map((_, i) => (
                  <div
                    key={i}
                    className="h-4 w-4 rounded-sm"
                    style={{ background: (Math.sin(i * 7.3 + 1.1) > 0.1) ? "oklch(0.65 0.2 145)" : "transparent" }}
                  />
                ))}
              </div>
              <div className="absolute inset-0 flex items-center justify-center">
                <div className="flex h-8 w-8 items-center justify-center rounded-md bg-[var(--window-bg)] text-[10px] font-bold text-primary">ER</div>
              </div>
            </div>

            <div className="text-center">
              <p className="text-[10px] text-muted-foreground mb-1">Your EDGE address</p>
              <p className="font-mono text-xs text-foreground break-all">{ADDRESS}</p>
            </div>

            <button
              onClick={handleCopy}
              className="flex items-center gap-2 rounded-lg bg-secondary px-4 py-2 text-xs font-medium text-foreground transition-colors hover:bg-secondary/70"
            >
              {copied ? <Check className="h-3.5 w-3.5 text-[var(--status-online)]" /> : <Copy className="h-3.5 w-3.5" />}
              {copied ? "Copied!" : "Copy address"}
            </button>

            <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60">
              <Shield className="h-3 w-3" />
              <span>Identity-bound to your fingerprint key</span>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
