"use client"

import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { Progress } from "@/components/ui/progress"
import { DashboardPanel } from "@/components/ui/dashboard-panel"
import {
  TestTube,
  CheckCircle2,
  XCircle,
  SkipForward,
} from "lucide-react"
import { atom, computed } from "nanostores"

export interface TestResult {
  id: string
  name: string
  status: "passed" | "failed" | "skipped" | "running" | "pending"
  duration?: number
  file?: string
  error?: string
  evidenceRef?: string
}

export interface TestSuite {
  suiteId: string
  name: string
  results: TestResult[]
  source: "real" | "demo"
  lastRun?: number
}

export interface TestState {
  suites: Map<string, TestSuite>
  totalPassed: number
  totalFailed: number
  totalSkipped: number
  coveragePct?: number
  lastRunTimestamp?: number
}

const initialTestState: TestState = {
  suites: new Map(),
  totalPassed: 0,
  totalFailed: 0,
  totalSkipped: 0,
}

export const testStore = atom<TestState>(initialTestState)

export const allSuites = computed(testStore, (s) =>
  Array.from(s.suites.values()),
)

export function updateTestSuite(suite: TestSuite): void {
  const state = testStore.get()
  const newSuites = new Map(state.suites)
  newSuites.set(suite.suiteId, suite)

  const allResults = Array.from(newSuites.values()).flatMap(s => s.results)
  testStore.set({
    suites: newSuites,
    totalPassed: allResults.filter(r => r.status === "passed").length,
    totalFailed: allResults.filter(r => r.status === "failed").length,
    totalSkipped: allResults.filter(r => r.status === "skipped").length,
    lastRunTimestamp: Date.now(),
  })
}

function SuiteCard({ suite }: { suite: TestSuite }) {
  const passed = suite.results.filter(r => r.status === "passed").length
  const failed = suite.results.filter(r => r.status === "failed").length
  const total = suite.results.length
  const pct = total > 0 ? Math.round((passed / total) * 100) : 0

  return (
    <div className="rounded-lg border border-border bg-card p-3 space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <TestTube className="h-3 w-3 text-muted-foreground" />
          <span className="text-xs font-medium">{suite.name}</span>
        </div>
        <div className="flex items-center gap-1">
          {suite.source === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1 py-0.5 text-[9px] text-yellow-400">
              Demo
            </span>
          )}
          <span className={cn(
            "rounded px-1.5 py-0.5 text-[9px] font-medium",
            failed > 0 ? "bg-red-500/20 text-red-400" : "bg-green-500/20 text-green-400"
          )}>
            {pct}%
          </span>
        </div>
      </div>

      <Progress value={pct} className="h-1" />

      <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
        <span className="flex items-center gap-1">
          <CheckCircle2 className="h-3 w-3 text-green-500" />
          {passed}
        </span>
        <span className="flex items-center gap-1">
          <XCircle className="h-3 w-3 text-red-500" />
          {failed}
        </span>
        <span className="flex items-center gap-1">
          <SkipForward className="h-3 w-3 text-gray-400" />
          {suite.results.filter(r => r.status === "skipped").length}
        </span>
      </div>

      {suite.results.filter(r => r.status === "failed").length > 0 && (
        <div className="space-y-1 max-h-24 overflow-y-auto">
          {suite.results.filter(r => r.status === "failed").map(r => (
            <div key={r.id} className="text-[9px] text-red-400 truncate">
              {r.name}: {r.error || "failed"}
            </div>
          ))}
        </div>
      )}

      {suite.lastRun && (
        <p className="text-[9px] text-muted-foreground">
          Last run: {new Date(suite.lastRun).toLocaleTimeString()}
        </p>
      )}
    </div>
  )
}

export function TestStatusPanel() {
  const suites = useStore(allSuites)
  const state = useStore(testStore)

  const total = state.totalPassed + state.totalFailed + state.totalSkipped
  const passPct = total > 0 ? Math.round((state.totalPassed / total) * 100) : 0

  return (
    <DashboardPanel
      title="Test Status"
      summary={`${state.totalPassed}/${total} passed (${passPct}%)`}
      empty={suites.length === 0}
      emptyMessage="No test suites"
      demoMessage="No real test data (demo mode)"
    >
      {state.coveragePct !== undefined && (
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center justify-between mb-1">
            <span className="text-xs text-muted-foreground">Coverage</span>
            <span className="text-xs font-medium">{state.coveragePct}%</span>
          </div>
          <Progress value={state.coveragePct} className="h-1.5" />
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {suites.map((suite) => (
          <SuiteCard key={suite.suiteId} suite={suite} />
        ))}
      </div>
    </DashboardPanel>
  )
}
