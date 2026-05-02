"use client"

import { useStore } from "@nanostores/react"
import { resourceStore, formatResourceSummary } from "@/platform/state/resource-store"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import {
  Cpu,
  MemoryStick,
  Network,
  Activity,
  GitBranch,
  FileText,
  Bot,
  TestTube,
  Package,
  Coins,
  AlertTriangle,
  CheckCircle2,
  Clock,
} from "lucide-react"
import { useEffect, useState } from "react"

interface ConkyMetricProps {
  icon: React.ReactNode
  label: string
  value: string
  sub?: string
  source?: string
  isStale?: boolean
  isDemo?: boolean
  onClick?: () => void
}

function ConkyMetric({ icon, label, value, sub, source, isStale, isDemo, onClick }: ConkyMetricProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs transition-colors",
        "hover:bg-white/5 text-left w-full",
        isStale && "opacity-50",
        isDemo && "border border-yellow-500/30"
      )}
      title={`${label}: ${value}${source ? ` (source: ${source})` : ""}`}
    >
      <div className="text-muted-foreground">{icon}</div>
      <div className="flex flex-col">
        <span className="font-medium text-foreground">{value}</span>
        <span className="text-[9px] text-muted-foreground">
          {label}{sub && ` · ${sub}`}
          {isDemo && " · Demo"}
        </span>
      </div>
      {source === "unknown" && (
        <AlertTriangle className="h-3 w-3 text-yellow-400 ml-auto" />
      )}
    </button>
  )
}

export function ConkyOverlay() {
  const resource = useStore(resourceStore)
  const agg = resource.aggregate
  const mode = getDashboardMode()
  const isDemo = mode === "demo"
  const [gitInfo, setGitInfo] = useState<{ branch: string; dirty: number } | null>(null)
  const [agentInfo, setAgentInfo] = useState<{ active: number; blocked: number } | null>(null)
  const [testInfo, setTestInfo] = useState<{ pass: number; fail: number } | null>(null)
  const [depInfo, setDepInfo] = useState<{ illegal: number } | null>(null)
  const [tokenInfo, setTokenInfo] = useState<{ today: string } | null>(null)

  // TODO: wire to real stores
  useEffect(() => {
    // These would come from git-store, agent-store, test-store, etc.
    setGitInfo({ branch: "main", dirty: 12 })
    setAgentInfo({ active: 3, blocked: 1 })
    setTestInfo({ pass: 842, fail: 3 })
    setDepInfo({ illegal: 1 })
    setTokenInfo({ today: "1.2M" })
  }, [])

  const isStale = (timestamp: number) => Date.now() - timestamp > 60000

  if (!agg) {
    return (
      <div className="fixed top-0 left-0 right-0 z-50 flex items-center gap-1 px-4 py-1 bg-[var(--bg)]/80 backdrop-blur-sm border-b border-border/50 text-xs">
        <span className="text-muted-foreground">EDGERUN</span>
        <span className="text-yellow-400 ml-auto">No resource data</span>
      </div>
    )
  }

  return (
    <div className="fixed top-0 left-0 right-0 z-50 flex items-center gap-1 px-4 py-1 bg-[var(--bg)]/80 backdrop-blur-sm border-b border-border/50 text-xs overflow-x-auto">
      <span className={cn(
        "font-medium text-foreground mr-2",
        isDemo && "text-yellow-400"
      )}>
        EDGERUN
        {isDemo && <span className="text-[9px] ml-1">(Demo)</span>}
      </span>

      <div className="flex items-center gap-1 flex-1 min-w-0 overflow-x-auto">
        {/* CPU */}
        <ConkyMetric
          icon={<Cpu className="h-3 w-3" />}
          label="CPU"
          value={`${agg.totalCores} cores · ${Math.round(agg.weightedCpuUtilization * 100)}%`}
          sub={agg.unhealthyNodes > 0 ? `${agg.unhealthyNodes} unhealthy` : undefined}
          source={resource.primarySource}
          isStale={false}
          isDemo={isDemo}
        />

        {/* Memory */}
        <ConkyMetric
          icon={<MemoryStick className="h-3 w-3" />}
          label="Mem"
          value={`${Math.round(agg.usedMemory / 1024 / 1024 / 1024 * 10) / 10}/${Math.round(agg.totalMemory / 1024 / 1024 / 1024 * 10) / 10} GB`}
          source={resource.primarySource}
          isStale={false}
          isDemo={isDemo}
        />

        {/* Nodes */}
        <ConkyMetric
          icon={<Network className="h-3 w-3" />}
          label="Nodes"
          value={`${agg.connectedNodes}/${agg.totalNodes} online`}
          sub={agg.unhealthyNodes > 0 ? `${agg.unhealthyNodes} unhealthy` : undefined}
          source={resource.primarySource}
          isStale={false}
          isDemo={isDemo}
        />

        {/* Jobs */}
        <ConkyMetric
          icon={<Activity className="h-3 w-3" />}
          label="Jobs"
          value={`${agg.activeJobs} active`}
          sub={agg.availableCapacity > 0 ? `${agg.availableCapacity} capacity` : undefined}
          source={resource.primarySource}
          isDemo={isDemo}
        />

        {/* Git */}
        {gitInfo && (
          <ConkyMetric
            icon={<GitBranch className="h-3 w-3" />}
            label="Git"
            value={gitInfo.branch}
            sub={`${gitInfo.dirty} dirty`}
          />
        )}

        {/* Tests */}
        {testInfo && (
          <ConkyMetric
            icon={<TestTube className="h-3 w-3" />}
            label="Tests"
            value={`${testInfo.pass} pass`}
            sub={testInfo.fail > 0 ? `${testInfo.fail} fail` : undefined}
          />
        )}

        {/* Dependencies */}
        {depInfo && depInfo.illegal > 0 && (
          <ConkyMetric
            icon={<Package className="h-3 w-3 text-red-400" />}
            label="Deps"
            value={`${depInfo.illegal} illegal`}
            sub="unapproved"
          />
        )}

        {/* Agents */}
        {agentInfo && (
          <ConkyMetric
            icon={<Bot className="h-3 w-3" />}
            label="Agents"
            value={`${agentInfo.active} active`}
            sub={agentInfo.blocked > 0 ? `${agentInfo.blocked} blocked` : undefined}
          />
        )}

        {/* Tokens */}
        {tokenInfo && (
          <ConkyMetric
            icon={<Coins className="h-3 w-3" />}
            label="Tokens"
            value={tokenInfo.today}
            sub="today"
          />
        )}

        {/* Alerts */}
        {agg.unhealthyNodes > 0 && (
          <span className="flex items-center gap-1 text-red-400 ml-2">
            <AlertTriangle className="h-3 w-3" />
            <span className="hidden sm:inline">{agg.unhealthyNodes} unhealthy</span>
          </span>
        )}
      </div>

      <div className="text-[9px] text-muted-foreground ml-2">
        {agg.sources.includes("unknown") ? "unknown" : agg.sources.join(", ")}
      </div>
    </div>
  )
}
