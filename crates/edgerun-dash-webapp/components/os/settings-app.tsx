"use client"

import { useState } from "react"
import {
  Palette, Shield, Network, Info, Bell, Cpu, Fingerprint,
  Globe, Wifi, WifiOff, ChevronRight, Check, RotateCcw,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { EdgerRunLogo } from "./edgerun-logo"

// ─── Types ───────────────────────────────────────────────────────────────────

type SettingsSection =
  | "appearance"
  | "privacy"
  | "network"
  | "notifications"
  | "performance"
  | "about"

// ─── Toggle ───────────────────────────────────────────────────────────────────

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      role="switch"
      aria-checked={value}
      onClick={() => onChange(!value)}
      className={cn(
        "relative h-5 w-9 rounded-full transition-colors duration-200",
        value ? "bg-primary" : "bg-secondary"
      )}
    >
      <span
        className={cn(
          "absolute top-0.5 h-4 w-4 rounded-full bg-foreground shadow transition-transform duration-200",
          value ? "translate-x-4" : "translate-x-0.5"
        )}
      />
    </button>
  )
}

// ─── Slider ───────────────────────────────────────────────────────────────────

function Slider({ value, onChange, min = 0, max = 100, label }: {
  value: number; onChange: (v: number) => void; min?: number; max?: number; label?: string
}) {
  return (
    <div className="flex items-center gap-3">
      {label && <span className="w-12 text-right font-mono text-[10px] text-muted-foreground">{value}{label}</span>}
      <input
        type="range" min={min} max={max} value={value}
        onChange={e => onChange(Number(e.target.value))}
        className="flex-1 accent-[var(--primary)] h-1"
      />
    </div>
  )
}

// ─── Row ─────────────────────────────────────────────────────────────────────

function Row({ label, sub, right }: { label: string; sub?: string; right: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between py-2.5">
      <div>
        <p className="text-sm text-foreground">{label}</p>
        {sub && <p className="text-[11px] text-muted-foreground">{sub}</p>}
      </div>
      {right}
    </div>
  )
}

// ─── Section panels ───────────────────────────────────────────────────────────

function AppearancePanel() {
  const [accentColor, setAccentColor] = useState("green")
  const [globeOpacity, setGlobeOpacity] = useState(40)
  const [windowBlur, setWindowBlur] = useState(true)
  const [animations, setAnimations] = useState(true)
  const [fontSize, setFontSize] = useState(13)
  const [dockPosition, setDockPosition] = useState("bottom")

  const accents = [
    { id: "green",  color: "oklch(0.65 0.2 145)" },
    { id: "blue",   color: "oklch(0.6 0.2 250)" },
    { id: "orange", color: "oklch(0.72 0.18 60)" },
    { id: "pink",   color: "oklch(0.65 0.22 340)" },
    { id: "cyan",   color: "oklch(0.68 0.16 200)" },
  ]

  return (
    <div className="space-y-1 divide-y divide-[var(--window-border)]">
      <div className="pb-4">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Accent Color</p>
        <div className="flex gap-2">
          {accents.map(a => (
            <button
              key={a.id}
              onClick={() => setAccentColor(a.id)}
              className="relative h-7 w-7 rounded-full transition-transform hover:scale-110"
              style={{ background: a.color }}
              title={a.id}
            >
              {accentColor === a.id && <Check className="absolute inset-0 m-auto h-3.5 w-3.5 text-black" />}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-0 divide-y divide-[var(--window-border)]">
        <Row label="Window blur" sub="Frosted glass effect on windows" right={<Toggle value={windowBlur} onChange={setWindowBlur} />} />
        <Row label="Animations" sub="Window open/close transitions" right={<Toggle value={animations} onChange={setAnimations} />} />
        <Row
          label="Globe opacity"
          sub={`Background globe visibility — ${globeOpacity}%`}
          right={<div className="w-32"><Slider value={globeOpacity} onChange={setGlobeOpacity} label="%" /></div>}
        />
        <Row
          label="Font size"
          sub={`UI base font size — ${fontSize}px`}
          right={<div className="w-32"><Slider value={fontSize} onChange={setFontSize} min={11} max={18} label="px" /></div>}
        />
        <Row
          label="Dock position"
          right={
            <div className="flex gap-1">
              {["bottom", "left", "right"].map(p => (
                <button
                  key={p}
                  onClick={() => setDockPosition(p)}
                  className={cn("rounded px-2 py-0.5 text-[11px] capitalize transition-colors",
                    dockPosition === p ? "bg-primary text-primary-foreground" : "bg-secondary text-muted-foreground hover:text-foreground"
                  )}
                >{p}</button>
              ))}
            </div>
          }
        />
      </div>
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
    <div className="space-y-0 divide-y divide-[var(--window-border)]">
      <div className="pb-3">
        <div className="flex items-center gap-2 rounded-lg bg-primary/10 p-3">
          <Fingerprint className="h-5 w-5 shrink-0 text-primary" />
          <div>
            <p className="text-sm font-medium text-foreground">Fingerprint locked</p>
            <p className="text-[11px] text-muted-foreground">WebAuthn credential active on this device</p>
          </div>
        </div>
      </div>
      <Row label="Biometric lock" sub="Require fingerprint to unlock" right={<Toggle value={biometricLock} onChange={setBiometricLock} />} />
      <Row label="Auto-lock" sub="Lock when idle" right={<Toggle value={autoLock} onChange={setAutoLock} />} />
      {autoLock && (
        <Row
          label="Auto-lock delay"
          sub={`${autoLockTime} minute${autoLockTime !== 1 ? "s" : ""}`}
          right={<div className="w-32"><Slider value={autoLockTime} onChange={setAutoLockTime} min={1} max={60} /></div>}
        />
      )}
      <div className="pt-2">
        <p className="mb-2 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Data & Privacy</p>
        <div className="space-y-0 divide-y divide-[var(--window-border)]">
          <Row label="Analytics" sub="Send anonymous usage data" right={<Toggle value={analytics} onChange={setAnalytics} />} />
          <Row label="Crash reports" sub="Share diagnostics on crash" right={<Toggle value={crashReports} onChange={setCrashReports} />} />
          <Row label="Location access" sub="Allow apps to access rough location" right={<Toggle value={locationAccess} onChange={setLocationAccess} />} />
        </div>
      </div>
    </div>
  )
}

function NetworkPanel() {
  const [vpn, setVpn] = useState(false)
  const [torRelay, setTorRelay] = useState(false)
  const [p2pDiscovery, setP2pDiscovery] = useState(true)
  const [bandwidth, setBandwidth] = useState(50)
  const [region, setRegion] = useState("auto")

  const regions = ["auto", "eu-west", "us-east", "ap-south", "us-west"]

  return (
    <div className="space-y-0 divide-y divide-[var(--window-border)]">
      {/* Live stats */}
      <div className="pb-4">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Live Status</p>
        <div className="grid grid-cols-3 gap-2">
          {[
            { label: "Latency", val: "14ms", color: "text-primary" },
            { label: "Peers", val: "12", color: "text-primary" },
            { label: "Upload", val: "2.4 MB/s", color: "text-[oklch(0.65_0.2_200)]" },
          ].map(s => (
            <div key={s.label} className="rounded-lg bg-secondary/40 p-2 text-center">
              <p className={cn("font-mono text-sm font-semibold", s.color)}>{s.val}</p>
              <p className="text-[10px] text-muted-foreground">{s.label}</p>
            </div>
          ))}
        </div>
      </div>

      <Row label="VPN tunnel" sub="Route traffic through Edgerun VPN" right={<Toggle value={vpn} onChange={setVpn} />} />
      <Row label="Tor relay" sub="Contribute as a relay node" right={<Toggle value={torRelay} onChange={setTorRelay} />} />
      <Row label="P2P discovery" sub="Allow peers to discover this node" right={<Toggle value={p2pDiscovery} onChange={setP2pDiscovery} />} />
      <Row
        label="Bandwidth limit"
        sub={`${bandwidth === 100 ? "Unlimited" : `${bandwidth}%`} of available bandwidth`}
        right={<div className="w-32"><Slider value={bandwidth} onChange={setBandwidth} label="%" /></div>}
      />
      <Row
        label="Region"
        right={
          <select
            value={region}
            onChange={e => setRegion(e.target.value)}
            className="rounded-md bg-secondary px-2 py-1 font-mono text-[11px] text-foreground outline-none"
          >
            {regions.map(r => <option key={r} value={r}>{r}</option>)}
          </select>
        }
      />
    </div>
  )
}

function NotificationsPanel() {
  const [systemAlerts, setSystemAlerts] = useState(true)
  const [chatNotifs, setChatNotifs] = useState(true)
  const [callNotifs, setCallNotifs] = useState(true)
  const [walletNotifs, setWalletNotifs] = useState(true)
  const [nodeEvents, setNodeEvents] = useState(false)
  const [sound, setSound] = useState(true)

  return (
    <div className="space-y-0 divide-y divide-[var(--window-border)]">
      <Row label="System alerts" sub="Critical runtime events" right={<Toggle value={systemAlerts} onChange={setSystemAlerts} />} />
      <Row label="Chat messages" sub="New messages from peers" right={<Toggle value={chatNotifs} onChange={setChatNotifs} />} />
      <Row label="Incoming calls" sub="P2P call notifications" right={<Toggle value={callNotifs} onChange={setCallNotifs} />} />
      <Row label="Wallet activity" sub="Transactions and transfers" right={<Toggle value={walletNotifs} onChange={setWalletNotifs} />} />
      <Row label="Node events" sub="Peer join / leave events" right={<Toggle value={nodeEvents} onChange={setNodeEvents} />} />
      <Row label="Sound" sub="Play audio for notifications" right={<Toggle value={sound} onChange={setSound} />} />
    </div>
  )
}

function PerformancePanel() {
  const [maxCpuUsage, setMaxCpuUsage] = useState(80)
  const [maxRamUsage, setMaxRamUsage] = useState(60)
  const [backgroundTasks, setBackgroundTasks] = useState(true)
  const [powerSaver, setPowerSaver] = useState(false)

  return (
    <div className="space-y-0 divide-y divide-[var(--window-border)]">
      <div className="pb-4">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-widest text-muted-foreground">Resource Limits</p>
        <div className="space-y-4">
          <div>
            <div className="mb-1 flex justify-between text-[11px]">
              <span className="text-muted-foreground">Max CPU usage</span>
              <span className="font-mono text-primary">{maxCpuUsage}%</span>
            </div>
            <Slider value={maxCpuUsage} onChange={setMaxCpuUsage} />
          </div>
          <div>
            <div className="mb-1 flex justify-between text-[11px]">
              <span className="text-muted-foreground">Max RAM allocation</span>
              <span className="font-mono text-primary">{maxRamUsage}%</span>
            </div>
            <Slider value={maxRamUsage} onChange={setMaxRamUsage} />
          </div>
        </div>
      </div>
      <Row label="Background tasks" sub="Allow tasks to run when minimized" right={<Toggle value={backgroundTasks} onChange={setBackgroundTasks} />} />
      <Row label="Power saver" sub="Reduce performance to save energy" right={<Toggle value={powerSaver} onChange={setPowerSaver} />} />
    </div>
  )
}

function AboutPanel() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col items-center gap-3 pt-2 text-center">
        <EdgerRunLogo size="lg" showWordmark={false} />
        <div>
          <p className="text-lg font-semibold tracking-tight text-foreground">Edgerun</p>
          <p className="text-sm text-muted-foreground">Distributed Runtime</p>
        </div>
      </div>

      <div className="space-y-0 divide-y divide-[var(--window-border)] rounded-lg border border-[var(--window-border)]">
        {[
          ["Version", "1.0.0-alpha"],
          ["Runtime", "WASI 2.0"],
          ["Node ID", "edge-0xdeadbeef"],
          ["Region", "eu-west"],
          ["Build", "2026.04.29"],
          ["Protocol", "WebRTC / CBOR"],
          ["Auth", "WebAuthn / FIDO2"],
        ].map(([k, v]) => (
          <div key={k} className="flex items-center justify-between px-3 py-2">
            <span className="text-xs text-muted-foreground">{k}</span>
            <span className="font-mono text-xs text-foreground">{v}</span>
          </div>
        ))}
      </div>

      <div className="flex gap-2">
        <button className="flex-1 rounded-lg border border-[var(--window-border)] px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground">
          Check for updates
        </button>
        <button className="flex-1 rounded-lg border border-[var(--window-border)] px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground">
          View changelog
        </button>
      </div>

      <p className="text-center font-mono text-[10px] text-muted-foreground/40">
        &copy; 2026 Edgerun. MIT License.
      </p>
    </div>
  )
}

// ─── Main ─────────────────────────────────────────────────────────────────────

const SECTIONS: { id: SettingsSection; label: string; icon: React.ReactNode }[] = [
  { id: "appearance", label: "Appearance", icon: <Palette className="h-4 w-4" /> },
  { id: "privacy",    label: "Privacy",    icon: <Shield className="h-4 w-4" /> },
  { id: "network",    label: "Network",    icon: <Globe className="h-4 w-4" /> },
  { id: "notifications", label: "Alerts",  icon: <Bell className="h-4 w-4" /> },
  { id: "performance", label: "Performance", icon: <Cpu className="h-4 w-4" /> },
  { id: "about",      label: "About",      icon: <Info className="h-4 w-4" /> },
]

export function SettingsApp() {
  const [active, setActive] = useState<SettingsSection>("appearance")

  const renderPanel = () => {
    switch (active) {
      case "appearance":    return <AppearancePanel />
      case "privacy":       return <PrivacyPanel />
      case "network":       return <NetworkPanel />
      case "notifications": return <NotificationsPanel />
      case "performance":   return <PerformancePanel />
      case "about":         return <AboutPanel />
    }
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Sidebar */}
      <div className="w-36 shrink-0 border-r border-[var(--window-border)] py-2">
        {SECTIONS.map(s => (
          <button
            key={s.id}
            onClick={() => setActive(s.id)}
            className={cn(
              "flex w-full items-center gap-2.5 px-3 py-2 text-xs transition-colors",
              active === s.id ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-secondary hover:text-foreground"
            )}
          >
            {s.icon}
            {s.label}
          </button>
        ))}
      </div>

      {/* Panel */}
      <div className="flex-1 overflow-auto p-4">
        <h2 className="mb-4 font-mono text-[11px] font-semibold uppercase tracking-widest text-muted-foreground">
          {SECTIONS.find(s => s.id === active)?.label}
        </h2>
        {renderPanel()}
      </div>
    </div>
  )
}
