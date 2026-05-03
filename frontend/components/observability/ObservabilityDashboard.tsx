"use client"

import { useStore } from "@nanostores/react"
import { resourceStore, allNodes } from "@/platform/state/resource-store"
import { getDashboardMode, isBootstrapTopology } from "@/platform/runtime/dashboard-mode"
import { ConkyOverlay } from "./ConkyOverlay"
import { cn } from "@/lib/utils"
import { ResourceOverview } from "./ResourceOverview"
import { NodeGrid } from "./NodeGrid"
import { PipelineProgress } from "../pipelines/PipelineProgress"
import { AgentActivityPanel } from "../agents/AgentActivityPanel"
import { TestStatusPanel } from "../tests/TestStatusPanel"
import { DependencyGraph } from "../dependencies/DependencyGraph"
import { TokenUsagePanel } from "../tokens/TokenUsagePanel"
import { AlertCenter } from "../alerts/AlertCenter"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs"
import {
  LayoutDashboard,
  GitBranch,
  TestTube,
  Bot,
  Package,
  Coins,
  AlertTriangle,
  Activity,
} from "lucide-react"
import { useState } from "react"

export function ObservabilityDashboard() {
  const resource = useStore(resourceStore)
  const nodes = useStore(allNodes)
  const mode = getDashboardMode()
  const isBootstrap = isBootstrapTopology()

  return (
    <div className="min-h-screen bg-background">
      {/* Conky-style overlay at top */}
      <ConkyOverlay />

      {/* Main content with top padding for overlay */}
      <div className="pt-12 p-4">
        <div className="max-w-7xl mx-auto space-y-4">
          {/* Header */}
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">
                EdgeRun Observability
                {mode === "demo" && (
                  <span className="ml-2 text-sm font-normal text-yellow-400">(Demo Mode)</span>
                )}
              </h1>
              <p className="text-sm text-muted-foreground">
                {isBootstrap
                  ? "Bootstrap topology — coordinator-assisted"
                  : "Decentralized operation"}
                {" · "}
                {mode === "real" ? "Connected to real nodes" : mode === "demo" ? "Demo data" : "Offline"}
              </p>
            </div>
          </div>

          {/* Tabs for different views */}
          <Tabs defaultValue="overview" className="space-y-4">
            <TabsList>
              <TabsTrigger value="overview">
                <LayoutDashboard className="h-4 w-4 mr-1" />
                Overview
              </TabsTrigger>
              <TabsTrigger value="nodes">
                <Activity className="h-4 w-4 mr-1" />
                Nodes ({nodes.length})
              </TabsTrigger>
              <TabsTrigger value="pipelines">
                <GitBranch className="h-4 w-4 mr-1" />
                Pipelines
              </TabsTrigger>
              <TabsTrigger value="agents">
                <Bot className="h-4 w-4 mr-1" />
                Agents
              </TabsTrigger>
              <TabsTrigger value="tests">
                <TestTube className="h-4 w-4 mr-1" />
                Tests
              </TabsTrigger>
              <TabsTrigger value="deps">
                <Package className="h-4 w-4 mr-1" />
                Dependencies
              </TabsTrigger>
              <TabsTrigger value="tokens">
                <Coins className="h-4 w-4 mr-1" />
                Tokens
              </TabsTrigger>
              <TabsTrigger value="alerts">
                <AlertTriangle className="h-4 w-4 mr-1" />
                Alerts
              </TabsTrigger>
            </TabsList>

            <TabsContent value="overview" className="space-y-4">
              <ResourceOverview />
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <PipelineProgress />
                <AgentActivityPanel />
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <TestStatusPanel />
                <TokenUsagePanel />
              </div>
              <DependencyGraph />
            </TabsContent>

            <TabsContent value="nodes">
              <NodeGrid />
            </TabsContent>

            <TabsContent value="pipelines">
              <PipelineProgress />
            </TabsContent>

            <TabsContent value="agents">
              <AgentActivityPanel />
            </TabsContent>

            <TabsContent value="tests">
              <TestStatusPanel />
            </TabsContent>

            <TabsContent value="deps">
              <DependencyGraph />
            </TabsContent>

            <TabsContent value="tokens">
              <TokenUsagePanel />
            </TabsContent>

            <TabsContent value="alerts">
              <AlertCenter />
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  )
}
