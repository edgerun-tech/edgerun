"use client"

import { useState, type ReactNode } from "react"
import { useStore } from "@nanostores/react"
import {
  Bell,
  Check,
  ChevronRight,
  Cpu,
  Fingerprint,
  Globe,
  Info,
  MonitorCog,
  Palette,
  RadioTower,
  RotateCcw,
  Shield,
  Sparkles,
  WalletCards,
} from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Switch } from "@/components/ui/switch"
import { CodelyzerNetworkPanel } from "@/components/sections/codelyzer-network"
import { cn } from "@/lib/utils"
import {
  resetUiSettings,
  uiSettingsStore,
  updateUiSettings,
  type UiAccent,
} from "@/stores/ui-settings-store"
import { EdgerunLogo } from "./edgerun-logo"

type SettingsSection =
  | "appearance"
  | "privacy"
  | "network"
  | "notifications"
  | "performance"
  | "about"

type SectionMeta = {
  id: SettingsSection
  label: string
  hint: string
  icon: ReactNode
}

const SECTIONS: SectionMeta[] = [
  { id: "appearance", label: "Appearance", hint: "Desktop, glow, dock", icon: <Palette className="h-4 w-4" /> },
  { id: "privacy", label: "Privacy", hint: "Identity and local data", icon: <Shield className="h-4 w-4" /> },
  { id: "network", label: "Network", hint: "Codelyzer bridge and graph links", icon: <Globe className="h-4 w-4" /> },
  { id: "notifications", label: "Alerts", hint: "System and app signals", icon: <Bell className="h-4 w-4" /> },
  { id: "performance", label: "Performance", hint: "Runtime policy", icon: <Cpu className="h-4 w-4" /> },
  { id: "about", label: "About", hint: "Build and runtime", icon: <Info className="h-4 w-4" /> },
]

function SettingsCard({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <Card className="gap-3 border-[var(--window-border)] bg-card/55 py-4 shadow-none">
      <CardHeader className="gap-1 px-4">
        <CardTitle className="text-sm">{title}</CardTitle>
        {description && <CardDescription className="text-xs">{description}</CardDescription>}
      </CardHeader>
      <CardContent className="px-4">
        {children}
      </CardContent>
    </Card>
  )
}

function SettingRow({ label, sub, right }: { label: string; sub?: string; right: ReactNode }) {
  return (
    <div className="flex min-h-11 items-center justify-between gap-4 border-t border-[var(--window-border)] py-3 first:border-t-0 first:pt-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-sm text-foreground">{label}</p>
        {sub && <p className="mt-0.5 text-[11px] leading-relaxed text-muted-foreground">{sub}</p>}
      </div>
      <div className="shrink-0">{right}</div>
    </div>
  )
}

function SliderControl({ value, onChange, min = 0, max = 100, suffix = "%" }: {
  value: number
  onChange: (v: number) => void
  min?: number
  max?: number
  suffix?: string
}) {
  return (
    <div className="flex w-40 items-center gap-3">
      <Slider value={[value]} min={min} max={max} onValueChange={([next]) => onChange(next ?? value)} />
      <span className="w-12 text-right font-mono text-[10px] text-primary">{value}{suffix}</span>
    </div>
  )
}

function AccentButton({ active, color, label, onClick }: { active: boolean; color: string; label: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "group relative h-10 flex-1 overflow-hidden rounded-xl border border-[var(--window-border)] transition-all hover:-translate-y-0.5 hover:border-primary/30",
        active && "border-primary/50 ring-2 ring-primary/20"
      )}
      title={label}
    >
      <span className="absolute inset-0" style={{ background: color }} />
      <span className="absolute inset-0 bg-gradient-to-br from-white/30 to-black/25" />
      {active && <Check className="absolute inset-0 m-auto h-4 w-4 text-black" />}
    </button>
  )
}

function AppearancePanel() {
  const settings = useStore(uiSettingsStore)

  const accents: Array<{ id: UiAccent; label: string; color: string }> = [
    { id: "green", label: "Edge", color: "oklch(0.65 0.2 145)" },
    { id: "blue", label: "Cloud", color: "oklch(0.6 0.2 250)" },
    { id: "orange", label: "Market", color: "oklch(0.72 0.18 60)" },
    { id: "pink", label: "Signal", color: "oklch(0.65 0.22 340)" },
    { id: "cyan", label: "Mesh", color: "oklch(0.68 0.16 200)" },
  ]

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_260px]">
      <div className="space-y-4">
        <SettingsCard title="Desktop" description="Keep the shell beautiful, calm, and useful.">
          <div className="space-y-3">
            <div className="grid grid-cols-5 gap-2">
              {accents.map(a => (
                <AccentButton
                  key={a.id}
                  active={settings.accent === a.id}
                  color={a.color}
                  label={a.label}
                  onClick={() => updateUiSettings({ accent: a.id })}
                />
              ))}
            </div>
            <SettingRow label="Window blur" sub="Frosted glass depth on app surfaces." right={<Switch checked={settings.windowBlur} onCheckedChange={(windowBlur) => updateUiSettings({ windowBlur })} size="sm" />} />
            <SettingRow label="Animations" sub="Window, dock, and xray motion." right={<Switch checked={settings.animations} onCheckedChange={(animations) => updateUiSettings({ animations })} size="sm" />} />
            <SettingRow label="Compact mode" sub="Tighter radius for dense app surfaces." right={<Switch checked={settings.compactMode} onCheckedChange={(compactMode) => updateUiSettings({ compactMode })} size="sm" />} />
          </div>
        </SettingsCard>

        <SettingsCard title="Layout" description="Tune density for the amount of work on screen.">
          <SettingRow label="Font size" sub="Base UI scale for compact desktop use." right={<SliderControl value={settings.fontSize} onChange={(fontSize) => updateUiSettings({ fontSize })} min={11} max={18} suffix="px" />} />
          <SettingRow
            label="Reset local UI"
            sub="Restore appearance, notification, and resource settings stored in this browser."
            right={
              <Button variant="outline" size="xs" onClick={resetUiSettings}>
                <RotateCcw className="h-3.5 w-3.5" />
                Reset
              </Button>
            }
          />
        </SettingsCard>
      </div>

      <Card className="overflow-hidden border-[var(--window-border)] bg-background/35 py-0 shadow-none">
        <div className="relative h-full min-h-[290px] p-4">
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_35%_25%,var(--primary)_0,transparent_28%),linear-gradient(135deg,transparent,rgba(255,255,255,0.04))] opacity-25" />
          <div className="relative flex h-full flex-col justify-between rounded-2xl border border-primary/10 bg-background/30 p-4">
            <div>
              <Badge variant="secondary" className="mb-4 gap-1.5 bg-primary/10 text-primary">
                <Sparkles className="h-3 w-3" /> Live preview
              </Badge>
              <p className="font-mono text-[10px] uppercase tracking-[0.28em] text-muted-foreground">xray workspace</p>
              <p className="mt-2 font-mono text-2xl text-foreground">graph-first</p>
              <p className="mt-1 text-xs text-muted-foreground">glowing panels · dock pages · app surfaces</p>
            </div>
            <div className="space-y-2 font-mono text-[10px] text-muted-foreground/60">
              <div className="flex justify-between"><span>accent</span><span className="text-primary">{settings.accent}</span></div>
              <div className="flex justify-between"><span>font</span><span>{settings.fontSize}px</span></div>
              <div className="h-1.5 rounded-full bg-secondary"><div className="h-full w-3/5 rounded-full bg-primary" /></div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  )
}

function PrivacyPanel() {
  const [biometricLock, setBiometricLock] = useState(true)
  const [autoLock, setAutoLock] = useState(true)
  const [autoLockTime, setAutoLockTime] = useState(5)
  const [analytics, setAnalytics] = useState(false)
  const [crashReports, setCrashReports] = useState(false)
  const [locationAccess, setLocationAccess] = useState(false)

  return (
    <div className="space-y-4">
      <Card className="border-primary/20 bg-primary/5 py-4 shadow-none">
        <CardContent className="flex items-center gap-3 px-4">
          <div className="rounded-xl bg-primary/10 p-3 text-primary">
            <Fingerprint className="h-5 w-5" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-foreground">Fingerprint locked</p>
            <p className="text-xs text-muted-foreground">WebAuthn credential active on this device.</p>
          </div>
          <Badge variant="outline" className="border-primary/20 text-primary">local</Badge>
        </CardContent>
      </Card>

      <SettingsCard title="Identity lock" description="Device trust should be clear and low-friction.">
        <SettingRow label="Biometric lock" sub="Require fingerprint or passkey to unlock." right={<Switch checked={biometricLock} onCheckedChange={setBiometricLock} size="sm" />} />
        <SettingRow label="Auto-lock" sub="Lock the shell when idle." right={<Switch checked={autoLock} onCheckedChange={setAutoLock} size="sm" />} />
        {autoLock && <SettingRow label="Auto-lock delay" sub={`${autoLockTime} minute${autoLockTime !== 1 ? "s" : ""}.`} right={<SliderControl value={autoLockTime} onChange={setAutoLockTime} min={1} max={60} suffix="m" />} />}
      </SettingsCard>

      <SettingsCard title="Data & privacy" description="Prefer local-first behavior unless explicitly enabled.">
        <SettingRow label="Analytics" sub="Send anonymous usage data." right={<Switch checked={analytics} onCheckedChange={setAnalytics} size="sm" />} />
        <SettingRow label="Crash reports" sub="Share diagnostics after crashes." right={<Switch checked={crashReports} onCheckedChange={setCrashReports} size="sm" />} />
        <SettingRow label="Location access" sub="Allow apps to request rough location." right={<Switch checked={locationAccess} onCheckedChange={setLocationAccess} size="sm" />} />
      </SettingsCard>
    </div>
  )
}

function NetworkPanel() {
  const [vpn, setVpn] = useState(false)
  const [relayMode, setRelayMode] = useState(false)
  const [p2pDiscovery, setP2pDiscovery] = useState(true)
  const [region, setRegion] = useState("auto")

  return (
    <div className="space-y-4">
      <CodelyzerNetworkPanel />

      <SettingsCard title="Routing" description="Separate trust from transport so hostile networks can still relay sealed packets.">
        <SettingRow label="VPN tunnel" sub="Route traffic through Edgerun VPN." right={<Switch checked={vpn} onCheckedChange={setVpn} size="sm" />} />
        <SettingRow label="Relay mode" sub="Contribute spare bandwidth as a paid relay." right={<Switch checked={relayMode} onCheckedChange={setRelayMode} size="sm" />} />
        <SettingRow label="P2P discovery" sub="Allow peers to discover this node." right={<Switch checked={p2pDiscovery} onCheckedChange={setP2pDiscovery} size="sm" />} />
        <SettingRow
          label="Region"
          sub="Use auto unless you need deterministic routing."
          right={
            <Select value={region} onValueChange={setRegion}>
              <SelectTrigger size="sm" className="w-36 font-mono text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {[
                  ["auto", "Auto"],
                  ["eu-west", "EU West"],
                  ["us-east", "US East"],
                  ["ap-south", "AP South"],
                  ["us-west", "US West"],
                ].map(([value, label]) => <SelectItem key={value} value={value}>{label}</SelectItem>)}
              </SelectContent>
            </Select>
          }
        />
      </SettingsCard>
    </div>
  )
}

function NotificationsPanel() {
  const settings = useStore(uiSettingsStore)

  return (
    <SettingsCard title="Notification policy" description="Keep alerts useful and quiet by default.">
      <SettingRow label="System alerts" sub="Critical runtime events." right={<Switch checked={settings.systemAlerts} onCheckedChange={(systemAlerts) => updateUiSettings({ systemAlerts })} size="sm" />} />
      <SettingRow label="Node events" sub="Peer join and leave events." right={<Switch checked={settings.nodeEvents} onCheckedChange={(nodeEvents) => updateUiSettings({ nodeEvents })} size="sm" />} />
      <SettingRow label="Sound" sub="Play audio for important notifications." right={<Switch checked={settings.sound} onCheckedChange={(sound) => updateUiSettings({ sound })} size="sm" />} />
    </SettingsCard>
  )
}

function PerformancePanel() {
  const settings = useStore(uiSettingsStore)

  return (
    <div className="space-y-4">
      <SettingsCard title="Resource limits" description="Apps should declare what they need; the shell decides what they get.">
        <SettingRow label="Max CPU usage" sub="Upper bound for foreground and background workloads." right={<SliderControl value={settings.maxCpuUsage} onChange={(maxCpuUsage) => updateUiSettings({ maxCpuUsage })} />} />
        <SettingRow label="Max RAM allocation" sub="Memory budget before apps must degrade or pause." right={<SliderControl value={settings.maxRamUsage} onChange={(maxRamUsage) => updateUiSettings({ maxRamUsage })} />} />
      </SettingsCard>

      <SettingsCard title="Runtime behavior" description="Balance local UX, battery, and compute-market participation.">
        <SettingRow label="Background tasks" sub="Allow tasks to run when minimized." right={<Switch checked={settings.backgroundTasks} onCheckedChange={(backgroundTasks) => updateUiSettings({ backgroundTasks })} size="sm" />} />
        <SettingRow label="Power saver" sub="Reduce performance to save energy." right={<Switch checked={settings.powerSaver} onCheckedChange={(powerSaver) => updateUiSettings({ powerSaver })} size="sm" />} />
      </SettingsCard>
    </div>
  )
}

function AboutPanel() {
  const details = [
    ["Version", "1.0.0-alpha"],
    ["Runtime", "WASI 2.0"],
    ["Node ID", "edge-0xdeadbeef"],
    ["Region", "eu-west"],
    ["Build", "2026.04.29"],
    ["Protocol", "WebRTC / CBOR"],
    ["Auth", "WebAuthn / FIDO2"],
  ]

  return (
    <div className="space-y-4">
      <Card className="overflow-hidden border-[var(--window-border)] bg-card/55 py-0 shadow-none">
        <CardContent className="relative flex items-center gap-4 px-5 py-5">
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,var(--primary)_0,transparent_28%)] opacity-10" />
          <div className="relative rounded-2xl border border-primary/10 bg-primary/5 p-3">
            <EdgerunLogo size="lg" variant="mark" />
          </div>
          <div className="relative min-w-0 flex-1">
            <p className="text-lg font-semibold tracking-tight text-foreground">Edgerun</p>
            <p className="text-sm text-muted-foreground">Distributed Runtime</p>
          </div>
          <Badge variant="secondary" className="relative bg-primary/10 text-primary">alpha</Badge>
        </CardContent>
      </Card>

      <SettingsCard title="Build information">
        <div className="divide-y divide-[var(--window-border)] rounded-xl border border-[var(--window-border)]">
          {details.map(([k, v]) => (
            <div key={k} className="flex items-center justify-between px-3 py-2">
              <span className="text-xs text-muted-foreground">{k}</span>
              <span className="font-mono text-xs text-foreground">{v}</span>
            </div>
          ))}
        </div>
      </SettingsCard>

      <div className="flex gap-2">
        <Button variant="outline" className="flex-1" size="sm"><RotateCcw className="h-3.5 w-3.5" /> Check for updates</Button>
        <Button variant="secondary" className="flex-1" size="sm">View changelog</Button>
      </div>

      <p className="text-center font-mono text-[10px] text-muted-foreground/40">
        &copy; 2026 Edgerun. MIT License.
      </p>
    </div>
  )
}

export function SettingsApp() {
  const [active, setActive] = useState<SettingsSection>("appearance")
  const section = SECTIONS.find(s => s.id === active) ?? SECTIONS[0]

  const renderPanel = () => {
    switch (active) {
      case "appearance": return <AppearancePanel />
      case "privacy": return <PrivacyPanel />
      case "network": return <NetworkPanel />
      case "notifications": return <NotificationsPanel />
      case "performance": return <PerformancePanel />
      case "about": return <AboutPanel />
    }
  }

  return (
    <div className="flex h-full overflow-hidden bg-background/40">
      <div className="w-56 shrink-0 border-r border-[var(--window-border)] bg-sidebar/35 p-3">
        <div className="mb-4 flex items-center gap-2 px-2">
          <MonitorCog className="h-4 w-4 text-primary" />
          <div>
            <p className="text-sm font-medium text-foreground">Settings</p>
            <p className="text-[10px] text-muted-foreground">System control</p>
          </div>
        </div>
        <div className="space-y-1">
          {SECTIONS.map(s => (
            <button
              key={s.id}
              onClick={() => setActive(s.id)}
              className={cn(
                "group flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors",
                active === s.id ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-secondary/65 hover:text-foreground"
              )}
            >
              <span className="shrink-0">{s.icon}</span>
              <span className="min-w-0 flex-1">
                <span className="block text-xs font-medium">{s.label}</span>
                <span className="block truncate text-[10px] text-muted-foreground">{s.hint}</span>
              </span>
              <ChevronRight className={cn("h-3.5 w-3.5 opacity-0 transition-opacity", active === s.id && "opacity-100")} />
            </button>
          ))}
        </div>
      </div>

      <div className="min-w-0 flex-1 overflow-auto p-5">
        <div className="mb-5 flex items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-semibold tracking-tight text-foreground">{section.label}</h2>
            <p className="text-sm text-muted-foreground">{section.hint}</p>
          </div>
          <Badge variant="outline" className="hidden gap-1.5 border-primary/20 text-primary sm:inline-flex">
            {active === "network" ? <RadioTower className="h-3 w-3" /> : active === "performance" ? <Cpu className="h-3 w-3" /> : active === "notifications" ? <Bell className="h-3 w-3" /> : active === "about" ? <Info className="h-3 w-3" /> : active === "privacy" ? <Shield className="h-3 w-3" /> : <WalletCards className="h-3 w-3" />}
            live policy
          </Badge>
        </div>
        {renderPanel()}
      </div>
    </div>
  )
}
