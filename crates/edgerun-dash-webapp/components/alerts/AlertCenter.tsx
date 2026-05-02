"use client"

import { useStore } from "@nanostores/react"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import {
  AlertTriangle,
  XCircle,
  AlertCircle,
  Info,
  CheckCircle2,
  X,
  Clock,
} from "lucide-react"
import { atom, computed } from "nanostores"

export type AlertSeverity = "critical" | "error" | "warning" | "info" | "resolved"

export interface Alert {
  alertId: string
  severity: AlertSeverity
  source: "node" | "coordinator" | "controller" | "agent" | "pipeline" | "test" | "system" | "demo"
  category: string // e.g., "provisioning", "resource", "connection", "security"
  title: string
  message: string
  nodeId?: string
  agentId?: string
  pipelineRunId?: string
  firstSeen: number
  lastSeen: number
  count: number
  isActive: boolean
  evidenceRefs: string[]
}

interface AlertState {
  alerts: Map<string, Alert>
  activeCount: number
  criticalCount: number
  errorCount: number
  warningCount: number
}

const initialAlertState: AlertState = {
  alerts: new Map(),
  activeCount: 0,
  criticalCount: 0,
  errorCount: 0,
  warningCount: 0,
}

export const alertStore = atom<AlertState>(initialAlertState)

export const activeAlerts = computed(alertStore, (s) =>
  Array.from(s.alerts.values()).filter((a: Alert) => a.isActive).sort((a: Alert, b: Alert) => b.lastSeen - a.lastSeen),
)

export function raiseAlert(alert: Omit<Alert, "alertId" | "firstSeen" | "lastSeen" | "count" | "isActive">): void {
  const state = alertStore.get()
  const id = `${alert.source}:${alert.category}:${alert.nodeId || alert.agentId || "system"}`
  const existing = state.alerts.get(id)
  
  if (existing) {
    const updated = {
      ...existing,
      lastSeen: Date.now(),
      count: existing.count + 1,
      isActive: true,
      message: alert.message, // update message
    }
    const newAlerts = new Map(state.alerts)
    newAlerts.set(id, updated)
    recomputeCounts(newAlerts)
  } else {
    const newAlert: Alert = {
      ...alert,
      alertId: id,
      firstSeen: Date.now(),
      lastSeen: Date.now(),
      count: 1,
      isActive: true,
    }
    const newAlerts = new Map(state.alerts)
    newAlerts.set(id, newAlert)
    recomputeCounts(newAlerts)
  }
}

export function resolveAlert(alertId: string): void {
  const state = alertStore.get()
  const alert = state.alerts.get(alertId)
  if (!alert) return
  const updated = { ...alert, isActive: false, lastSeen: Date.now() }
  const newAlerts = new Map(state.alerts)
  newAlerts.set(alertId, updated)
  recomputeCounts(newAlerts)
}

function recomputeCounts(alerts: Map<string, Alert>): void {
  const arr = Array.from(alerts.values())
  alertStore.set({
    alerts,
    activeCount: arr.filter(a => a.isActive).length,
    criticalCount: arr.filter(a => a.isActive && a.severity === "critical").length,
    errorCount: arr.filter(a => a.isActive && a.severity === "error").length,
    warningCount: arr.filter(a => a.isActive && a.severity === "warning").length,
  })
}

const SEVERITY_ICONS: Record<AlertSeverity, React.ReactNode> = {
  critical: <XCircle className="h-4 w-4 text-red-600" />,
  error: <XCircle className="h-4 w-4 text-red-500" />,
  warning: <AlertTriangle className="h-4 w-4 text-yellow-500" />,
  info: <Info className="h-4 w-4 text-blue-500" />,
  resolved: <CheckCircle2 className="h-4 w-4 text-green-500" />,
}

const SEVERITY_COLORS: Record<AlertSeverity, string> = {
  critical: "border-red-600/30 bg-red-600/5",
  error: "border-red-500/30 bg-red-500/5",
  warning: "border-yellow-500/30 bg-yellow-500/5",
  info: "border-blue-500/30 bg-blue-500/5",
  resolved: "border-green-500/30 bg-green-500/5",
}

function AlertCard({ alert }: { alert: Alert }) {
  const icon = SEVERITY_ICONS[alert.severity]
  const colorClass = SEVERITY_COLORS[alert.severity]
  
  return (
    <div className={cn("rounded border p-3 space-y-1", colorClass)}>
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          {icon}
          <span className="text-xs font-medium">{alert.title}</span>
        </div>
        <div className="flex items-center gap-1">
          {alert.source === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1 py-0.5 text-[9px] text-yellow-400">
              Demo
            </span>
          )}
          <span className="rounded bg-muted px-1 py-0.5 text-[9px]">
            {alert.category}
          </span>
          {alert.count > 1 && (
            <span className="rounded bg-muted px-1 py-0.5 text-[9px]">
              x{alert.count}
            </span>
          )}
        </div>
      </div>
      
      <p className="text-[10px] text-muted-foreground">{alert.message}</p>
      
      <div className="flex items-center gap-3 text-[9px] text-muted-foreground">
        <span>Source: {alert.source}</span>
        {alert.nodeId && <span>Node: {alert.nodeId.slice(0, 12)}</span>}
        {alert.agentId && <span>Agent: {alert.agentId}</span>}
        <span>{new Date(alert.lastSeen).toLocaleTimeString()}</span>
      </div>
      
      {alert.evidenceRefs.length > 0 && (
        <p className="text-[9px] text-muted-foreground">
          {alert.evidenceRefs.length} evidence refs
        </p>
      )}
    </div>
  )
}

export function AlertCenter() {
  const alerts = useStore(activeAlerts)
  const state = useStore(alertStore)
  const mode = getDashboardMode()

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">Alert Center</h3>
        <div className="flex items-center gap-2">
          {mode === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo
            </span>
          )}
          <span className="text-xs text-muted-foreground">
            {state.activeCount} active · {state.criticalCount} critical
          </span>
        </div>
      </div>

      {state.criticalCount > 0 && (
        <div className="rounded border border-red-600/30 bg-red-600/5 p-2 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-red-600" />
          <span className="text-xs text-red-400">
            {state.criticalCount} critical alert(s) require attention
          </span>
        </div>
      )}

      {alerts.length === 0 && (
        <div className="rounded-lg border border-border bg-card p-4 text-center">
          <CheckCircle2 className="h-8 w-8 text-green-500 mx-auto mb-2" />
          <p className="text-sm text-muted-foreground">
            {mode === "demo" ? "No real alerts (demo mode)" : "No active alerts"}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 gap-2">
        {alerts.map((alert: Alert) => (
          <AlertCard key={alert.alertId} alert={alert} />
        ))}
      </div>
    </div>
  )
}
