"use client"

import { useStore } from "@nanostores/react"
import { agentStore, allAgents, activeAgents, type AgentInfo, type AgentStatus } from "@/platform/state/agent-store"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import {
  Bot,
  CheckCircle2,
  XCircle,
  Loader2,
  Clock,
  ShieldCheck,
  AlertTriangle,
  Skull,
  Ban,
} from "lucide-react"

const STATUS_ICONS: Record<AgentStatus, React.ReactNode> = {
  idle: <Clock className="h-3 w-3 text-muted-foreground" />,
  planning: <Bot className="h-3 w-3 text-blue-400" />,
  editing: <Bot className="h-3 w-3 text-yellow-400" />,
  testing: <Loader2 className="h-3 w-3 animate-spin text-blue-500" />,
  waiting_approval: <ShieldCheck className="h-3 w-3 text-yellow-500" />,
  blocked: <Ban className="h-3 w-3 text-red-500" />,
  claimed_done: <CheckCircle2 className="h-3 w-3 text-green-500" />,
  verified_done: <CheckCircle2 className="h-3 w-3 text-green-600" />,
  failed: <XCircle className="h-3 w-3 text-red-500" />,
  abandoned: <Skull className="h-3 w-3 text-gray-400" />,
}

function AgentCard({ agent }: { agent: AgentInfo }) {
  const icon = STATUS_ICONS[agent.status] || STATUS_ICONS.idle
  const isVerified = agent.isClaimVerified
  const hasClaim = agent.claimedCompletion

  return (
    <div className={cn(
      "rounded-lg border p-3 space-y-2",
      agent.status === "blocked" && "border-red-500/30 bg-red-500/5",
      agent.status === "claimed_done" && !isVerified && "border-yellow-500/30 bg-yellow-500/5",
      agent.status === "verified_done" && "border-green-500/30 bg-green-500/5",
      agent.status === "failed" && "border-red-500/30 bg-red-500/5",
    )}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {icon}
          <span className="text-xs font-medium">{agent.agentId}</span>
        </div>
        <div className="flex items-center gap-1">
          {hasClaim && !isVerified && (
            <AlertTriangle className="h-3 w-3 text-yellow-500" aria-label="Claimed done but not verified" />
          )}
          {isVerified && (
            <ShieldCheck className="h-3 w-3 text-green-500" aria-label="Verified done" />
          )}
          <span className="rounded bg-muted px-1.5 py-0.5 text-[9px] font-medium">
            {agent.status}
          </span>
        </div>
      </div>

      {agent.model && (
        <p className="text-[10px] text-muted-foreground">{agent.model} {agent.provider ? `(${agent.provider})` : ""}</p>
      )}

      {agent.currentFiles.length > 0 && (
        <div className="text-[10px] text-muted-foreground">
          <span className="font-medium">Files:</span> {agent.currentFiles.slice(0, 3).join(", ")}
          {agent.currentFiles.length > 3 && ` +${agent.currentFiles.length - 3} more`}
        </div>
      )}

      {agent.gitBranch && (
        <p className="text-[10px] text-muted-foreground flex items-center gap-1">
          <span>🌿</span> {agent.gitBranch}
        </p>
      )}

      <p className="text-[10px] text-muted-foreground truncate">{agent.lastMessage}</p>

      <div className="flex items-center gap-2 text-[9px] text-muted-foreground">
        <span>Token usage: {agent.tokenUsage ? (agent.tokenUsage.total || 0).toLocaleString() : 0}</span>
        {agent.testsRun && <span className="text-green-400">Tests run</span>}
        {!agent.testsRun && agent.status === "claimed_done" && (
          <span className="text-yellow-400">No tests run</span>
        )}
      </div>

      {agent.evidenceRefs.length > 0 && (
        <p className="text-[9px] text-muted-foreground">
          {agent.evidenceRefs.length} evidence refs
        </p>
      )}

      {agent.errors.length > 0 && (
        <div className="text-[9px] text-red-400">
          {agent.errors.map((err, i) => (
            <p key={i} className="truncate">{err}</p>
          ))}
        </div>
      )}
    </div>
  )
}

export function AgentActivityPanel() {
  const agents = useStore(allAgents)
  const active = useStore(activeAgents)
  const store = useStore(agentStore)
  const mode = getDashboardMode()

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">Agent Activity</h3>
        <div className="flex items-center gap-2">
          {mode === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo
            </span>
          )}
          <span className="text-xs text-muted-foreground">
            {store.activeCount} active · {store.claimedDoneCount} claimed · {store.verifiedDoneCount} verified
          </span>
        </div>
      </div>

      {agents.length === 0 && (
        <div className="rounded-lg border border-border bg-card p-4 text-center">
          <p className="text-sm text-muted-foreground">
            {mode === "demo" ? "No real agent data (demo mode)" : "No active agents"}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {agents.map((agent) => (
          <AgentCard key={agent.agentId} agent={agent} />
        ))}
      </div>

      {store.claimedDoneCount > store.verifiedDoneCount && (
        <div className="rounded border border-yellow-500/30 bg-yellow-500/5 p-2">
          <p className="text-xs text-yellow-400 flex items-center gap-1">
            <AlertTriangle className="h-3 w-3" />
            {store.claimedDoneCount - store.verifiedDoneCount} agent(s) claimed done but not verified
          </p>
        </div>
      )}
    </div>
  )
}
