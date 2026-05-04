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
import { FinancesApp } from "@/components/os/finances-app"
import { CodelyzerNetworkPanel } from "@/components/sections/codelyzer-network"
import { XrayWorkspace } from "@/features/xray"
import type { AppSurfaceDef } from "@/stores/desktop-store"

export type ComponentPreviewGroup =
  | "components/ui"
  | "layouts"
  | "sections"
  | "apps"
  | "features/xray"
  | "review/cleanup"

export type ComponentPreview = {
  id: string
  title: string
  group: ComponentPreviewGroup
  description: string
  status: "canonical" | "candidate" | "review" | "scaffold"
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

function FinancesPreview() {
  return <PanelFrame><FinancesApp /></PanelFrame>
}

function CodelyzerNetworkPreview() {
  return <PanelFrame><CodelyzerNetworkPanel /></PanelFrame>
}

function XrayWorkspacePreview() {
  return <PanelFrame><XrayWorkspace /></PanelFrame>
}

export const COMPONENT_PREVIEWS: ComponentPreview[] = [
  {
    id: "xray-desktop-surface",
    title: "Xray desktop surface",
    group: "layouts",
    description: "Canonical desktop stage. Graph in the center, responsive metrics around it.",
    status: "canonical",
    component: XrayDesktopPreview,
  },
  {
    id: "app-overlay-host",
    title: "App overlay host",
    group: "layouts",
    description: "Chrome-less almost-fullscreen app surface with outside-click dismissal.",
    status: "canonical",
    component: AppOverlayPreview,
  },
  {
    id: "settings-app",
    title: "Settings app",
    group: "apps",
    description: "Production app surface. Should move toward shared app-shell primitives.",
    status: "candidate",
    component: SettingsPreview,
  },
  {
    id: "finances-app",
    title: "Finances app",
    group: "apps",
    description: "Finance hub for portfolio, wallet transfers, exchange, rewards, and settlement.",
    status: "canonical",
    component: FinancesPreview,
  },
  {
    id: "codelyzer-network-panel",
    title: "Codelyzer network panel",
    group: "sections",
    description: "Lists active repository graph connections from the local codelyzer bridge.",
    status: "canonical",
    component: CodelyzerNetworkPreview,
  },
  {
    id: "xray-workspace",
    title: "Xray workspace",
    group: "features/xray",
    description: "Canonical standalone xray feature page surface.",
    status: "canonical",
    component: XrayWorkspacePreview,
  },
  {
    id: "ui-primitives",
    title: "UI primitives",
    group: "components/ui",
    description: "Shared primitives only. These should not import app state.",
    status: "canonical",
    component: UiPrimitivePreview,
  },
]

export const COMPONENT_GROUP_LABELS: Record<ComponentPreviewGroup, string> = {
  "components/ui": "Components / UI",
  layouts: "Layouts",
  sections: "Sections",
  apps: "Apps",
  "features/xray": "Feature / Xray",
  "review/cleanup": "Review / Cleanup",
}
