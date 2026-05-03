"use client"

import { useStore } from "@nanostores/react"
import { pipelineStore, allRuns, activeRuns, type PipelineRun, type StepStatus } from "@/platform/state/pipeline-store"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import { Progress } from "@/components/ui/progress"
import {
  GitBranch,
  CheckCircle2,
  XCircle,
  Loader2,
  Clock,
  Ban,
  SkipForward,
  ShieldCheck,
  AlertTriangle,
} from "lucide-react"

const STEP_ICONS: Record<StepStatus, React.ReactNode> = {
  pending: <Clock className="h-3 w-3 text-muted-foreground" />,
  ready: <Clock className="h-3 w-3 text-blue-400" />,
  running: <Loader2 className="h-3 w-3 animate-spin text-blue-500" />,
  waiting_approval: <ShieldCheck className="h-3 w-3 text-yellow-500" />,
  waiting_external: <Clock className="h-3 w-3 text-gray-400" />,
  blocked: <Ban className="h-3 w-3 text-red-500" />,
  succeeded: <CheckCircle2 className="h-3 w-3 text-green-500" />,
  failed: <XCircle className="h-3 w-3 text-red-500" />,
  skipped: <SkipForward className="h-3 w-3 text-gray-400" />,
  cancelled: <Ban className="h-3 w-3 text-gray-400" />,
  verifying: <ShieldCheck className="h-3 w-3 animate-pulse text-yellow-500" />,
  verified: <CheckCircle2 className="h-3 w-3 text-green-600" />,
}

function StepRow({ step, runId }: { step: PipelineRun["steps"][0]; runId: string }) {
  const icon = STEP_ICONS[step.status] || STEP_ICONS.pending
  return (
    <div className="flex items-center gap-2 text-xs py-1">
      {icon}
      <span className="flex-1 truncate">{step.name}</span>
      {step.status === "running" && step.duration && (
        <span className="text-muted-foreground">{Math.round(step.duration / 1000)}s</span>
      )}
      {step.evidenceRefs.length > 0 && (
        <span className="rounded bg-muted px-1 text-[9px]">{step.evidenceRefs.length} evidence</span>
      )}
    </div>
  )
}

function RunCard({ run }: { run: PipelineRun }) {
  const isBlocked = run.status === "blocked"
  const isCompleted = run.status === "completed"
  const isFailed = run.status === "failed"

  return (
    <div className={cn(
      "rounded-lg border p-3 space-y-2",
      isBlocked && "border-yellow-500/30 bg-yellow-500/5",
      isFailed && "border-red-500/30 bg-red-500/5",
      isCompleted && "border-green-500/30 bg-green-500/5",
    )}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {run.status === "running" && <Loader2 className="h-3 w-3 animate-spin text-blue-500" />}
          {isCompleted && <CheckCircle2 className="h-3 w-3 text-green-500" />}
          {isFailed && <XCircle className="h-3 w-3 text-red-500" />}
          {isBlocked && <AlertTriangle className="h-3 w-3 text-yellow-500" />}
          <span className="text-xs font-medium">{run.runId}</span>
        </div>
        <span className="rounded bg-muted px-1.5 py-0.5 text-[9px] font-medium">
          {run.status}
        </span>
      </div>

      {run.trigger && (
        <p className="text-[10px] text-muted-foreground">Trigger: {run.trigger}</p>
      )}

      <Progress value={run.percentComplete} className="h-1.5" />

      <div className="space-y-0.5 max-h-32 overflow-y-auto">
        {run.steps.map((step) => (
          <StepRow key={step.stepId} step={step} runId={run.runId} />
        ))}
      </div>

      {run.blockedReason && (
        <p className="text-[10px] text-yellow-400">Blocked: {run.blockedReason}</p>
      )}

      {run.artifacts.length > 0 && (
        <p className="text-[10px] text-muted-foreground">
          {run.artifacts.length} artifacts
        </p>
      )}
    </div>
  )
}

export function PipelineProgress() {
  const runs = useStore(allRuns)
  const active = useStore(activeRuns)
  const store = useStore(pipelineStore)
  const mode = getDashboardMode()

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">Pipeline Progress</h3>
        <div className="flex items-center gap-2">
          {mode === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo
            </span>
          )}
          <span className="text-xs text-muted-foreground">
            {store.activeCount} active · {store.blockedCount} blocked
          </span>
        </div>
      </div>

      {runs.length === 0 && (
        <div className="rounded-lg border border-border bg-card p-4 text-center">
          <p className="text-sm text-muted-foreground">
            {mode === "demo" ? "No real pipeline data (demo mode)" : "No pipeline runs"}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {runs.map((run) => (
          <RunCard key={run.runId} run={run} />
        ))}
      </div>
    </div>
  )
}
