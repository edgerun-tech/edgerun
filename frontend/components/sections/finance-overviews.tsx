"use client"

import { ArrowDownLeft, ArrowUpRight, TrendingUp, WalletCards } from "lucide-react"
import { cn } from "@/lib/utils"

const ASSET_COLORS: Record<string, string> = {
  EDGE: "#22c55e",
  USDT: "#26a17b",
  BTC: "#f7931a",
  ETH: "#627eea",
  SOL: "#9945ff",
}

const portfolioAssets = [
  { symbol: "EDGE", value: 247.61, amount: "134.57", fill: ASSET_COLORS.EDGE },
  { symbol: "USDT", value: 92.0, amount: "92.00", fill: ASSET_COLORS.USDT },
  { symbol: "ETH", value: 38.5, amount: "0.012", fill: ASSET_COLORS.ETH },
  { symbol: "BTC", value: 0, amount: "0.0000", fill: ASSET_COLORS.BTC },
  { symbol: "SOL", value: 0, amount: "0.00", fill: ASSET_COLORS.SOL },
]

const recentActivity = [
  { label: "Compute reward", amount: 4.82, type: "in" as const },
  { label: "Paid Elias Voss", amount: -2.0, type: "out" as const },
]

function formatUsd(value: number) {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  }).format(value)
}

function MiniMetric({ label, value, tone }: { label: string; value: string; tone?: "good" | "muted" }) {
  return (
    <div className="rounded-xl border border-border/70 bg-background/55 p-2">
      <div className="text-[9px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className={cn("mt-1 truncate font-mono text-xs text-foreground", tone === "good" && "text-[var(--status-online)]", tone === "muted" && "text-muted-foreground")}>{value}</div>
    </div>
  )
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

export function FinancesOverviewWidget() {
  const chartData = portfolioAssets.filter((asset) => asset.value > 0)
  const totalValue = chartData.reduce((acc, asset) => acc + asset.value, 0)
  const edgeBalance = portfolioAssets.find((asset) => asset.symbol === "EDGE")?.amount ?? "0.00"
  const projectedMonthly = totalValue * 0.052
  const dayChange = 4.82 - 2.0

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background/5 p-3 text-foreground">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-xs font-semibold">
            <WalletCards className="h-3.5 w-3.5 text-primary" />
            Finances
          </div>
          <div className="mt-0.5 text-[10px] text-muted-foreground">Portfolio overview</div>
        </div>
        <div className="rounded-full border border-[var(--status-online)]/20 bg-[var(--status-online)]/10 px-2 py-0.5 font-mono text-[10px] text-[var(--status-online)]">
          +{dayChange.toFixed(2)} EDGE
        </div>
      </div>

      <div className="relative mx-auto flex h-[122px] w-[122px] shrink-0 items-center justify-center">
        <div
          className="flex h-[104px] w-[104px] items-center justify-center rounded-full"
          style={{ background: `conic-gradient(${donutGradient(chartData)})` }}
        >
          <div className="flex h-[72px] w-[72px] flex-col items-center justify-center rounded-full bg-background">
            <span className="text-[15px] font-bold text-foreground">{formatUsd(totalValue)}</span>
            <span className="text-[9px] text-muted-foreground">total</span>
          </div>
        </div>
      </div>

      <div className="mt-2 grid grid-cols-2 gap-2">
        <MiniMetric label="EDGE" value={edgeBalance} />
        <MiniMetric label="Monthly" value={formatUsd(projectedMonthly)} tone="good" />
      </div>

      <div className="mt-2 min-h-0 flex-1 space-y-1 overflow-hidden">
        {recentActivity.map((item) => (
          <div key={item.label} className="flex items-center gap-2 rounded-lg bg-background/45 px-2 py-1.5 text-[10px]">
            <span className={cn("flex h-5 w-5 shrink-0 items-center justify-center rounded-full", item.type === "in" ? "bg-[var(--status-online)]/10 text-[var(--status-online)]" : "bg-[var(--status-error)]/10 text-[var(--status-error)]")}>{item.type === "in" ? <ArrowDownLeft className="h-3 w-3" /> : <ArrowUpRight className="h-3 w-3" />}</span>
            <span className="min-w-0 flex-1 truncate text-muted-foreground">{item.label}</span>
            <span className={cn("font-mono", item.amount > 0 && "text-[var(--status-online)]")}>{item.amount > 0 ? "+" : ""}{item.amount.toFixed(2)}</span>
          </div>
        ))}
      </div>

      <div className="mt-2 flex items-center gap-1.5 text-[10px] text-muted-foreground">
        <TrendingUp className="h-3 w-3 text-[var(--status-online)]" />
        Rewards active · settlement ready
      </div>
    </div>
  )
}
