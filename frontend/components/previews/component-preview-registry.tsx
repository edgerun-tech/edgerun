"use client"

import type { ComponentType, ReactNode } from "react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Switch } from "@/components/ui/switch"
import { Slider } from "@/components/ui/slider"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { XrayDesktopSurface } from "@/components/os/xray-desktop-surface"
import { AppOverlayHost } from "@/components/os/app-overlay-host"
import { SettingsApp } from "@/components/os/settings-app"
import { WalletApp } from "@/components/os/wallet-app"
import { ResourceMonitor } from "@/components/os/resource-monitor"
import { XrayWorkspace } from "@/features/xray"
import type { AppSurfaceDef } from "@/stores/desktop-store"

export type ComponentPreviewGroup =
  | "production/os"
  | "production/xray"
  | "production/ui"
  | "scaffolding/observability"
  | "legacy/review"

export type ComponentPreview = {
  id: string
  title: string
  group: ComponentPreviewGroup
  description: string
  status: "canonical" | "candidate" | "legacy" | "scaffold"
  component: ComponentType
}

function UiPrimitivePreview() {
  return (
    <div className="space-y-4 p-4">
      <Card>
        <CardHeader>
          <CardTitle>UI primitives</CardTitle>
          <CardDescription>Shared shadcn-style components used by production surfaces.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button>Primary</Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="outline">Outline</Button>
            <Badge>Badge</Badge>
            <Badge variant="outline">Outline badge</Badge>
          </div>
          <div className="flex max-w-sm items-center gap-4">
            <Switch defaultChecked />
            <Slider defaultValue={[62]} />
          </div>
          <Tabs defaultValue="one" className="max-w-md">
            <TabsList>
              <TabsTrigger value="one">One</TabsTrigger>
              <TabsTrigger value="two">Two</TabsTrigger>
            </TabsList>
            <TabsContent value="one" className="text-sm text-muted-foreground">First tab content.</TabsContent>
            <TabsContent value="two" className="text-sm text-muted-foreground">Second tab content.</TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  )
}

function XrayDesktopPreview() {
  return (
    <div className="relative h-[720px] overflow-hidden rounded-3xl border bg-background">
      <XrayDesktopSurface
        nodeCount={12}
        activeSessions={3}
        ramUsage={{ used: 4.2, total: 8 }}
        runningApps={2}
        isConnected
      />
    </div>
  )
}

function AppOverlayPreview() {
  const surface: AppSurfaceDef = {
    id: "preview-settings",
    appId: "settings",
    title: "Settings",
    icon: null,
    kind: "overlay",
    dismissOnOutsideClick: true,
    defaultSize: { width: 960, height: 700 },
    component: <SettingsApp />,
  }

  return (
    <div className="relative h-[720px] overflow-hidden rounded-3xl border bg-zinc-950">
      <XrayDesktopSurface
        nodeCount={8}
        activeSessions={1}
        ramUsage={{ used: 2.6, total: 8 }}
        runningApps={1}
        isConnected
      />
      <AppOverlayHost
        surfaces={[surface]}
        surfaceOrder={[surface.id]}
        focusedSurfaceId={surface.id}
        onFocus={() => undefined}
        onClose={() => undefined}
      />
    </div>
  )
}

function PanelFrame({ children }: { children: ReactNode }) {
  return (
    <div className="h-[640px] overflow-hidden rounded-3xl border bg-background">
      {children}
    </div>
  )
}

function SettingsPreview() {
  return <PanelFrame><SettingsApp /></PanelFrame>
}

function WalletPreview() {
  return <PanelFrame><WalletApp /></PanelFrame>
}

function ResourceMonitorPreview() {
  return <PanelFrame><ResourceMonitor /></PanelFrame>
}

function XrayWorkspacePreview() {
  return <PanelFrame><XrayWorkspace /></PanelFrame>
}

export const COMPONENT_PREVIEWS: ComponentPreview[] = [
  {
    id: "xray-desktop-surface",
    title: "Xray desktop surface",
    group: "production/os",
    description: "Canonical desktop stage. Graph in the center, responsive metrics around it.",
    status: "canonical",
    component: XrayDesktopPreview,
  },
  {
    id: "app-overlay-host",
    title: "App overlay host",
    group: "production/os",
    description: "Chrome-less almost-fullscreen app surface with outside-click dismissal.",
    status: "canonical",
    component: AppOverlayPreview,
  },
  {
    id: "settings-app",
    title: "Settings app",
    group: "production/os",
    description: "Production settings surface. Should move toward shared app-shell primitives.",
    status: "candidate",
    component: SettingsPreview,
  },
  {
    id: "wallet-app",
    title: "Wallet app",
    group: "production/os",
    description: "Production wallet surface candidate for sidebar/app-shell consolidation.",
    status: "candidate",
    component: WalletPreview,
  },
  {
    id: "resource-monitor",
    title: "Resource monitor",
    group: "scaffolding/observability",
    description: "Candidate pinned-widget content. Should not own desktop layout itself.",
    status: "candidate",
    component: ResourceMonitorPreview,
  },
  {
    id: "xray-workspace",
    title: "Xray workspace",
    group: "production/xray",
    description: "Canonical standalone xray feature page surface.",
    status: "canonical",
    component: XrayWorkspacePreview,
  },
  {
    id: "ui-primitives",
    title: "UI primitives",
    group: "production/ui",
    description: "Shared primitives only. These should not import app state.",
    status: "canonical",
    component: UiPrimitivePreview,
  },
]

export const COMPONENT_GROUP_LABELS: Record<ComponentPreviewGroup, string> = {
  "production/os": "Production OS",
  "production/xray": "Production Xray",
  "production/ui": "UI Primitives",
  "scaffolding/observability": "Observability Candidates",
  "legacy/review": "Legacy / Review",
}
