"use client"

import { useState } from "react"
import { ChevronLeft, ChevronRight, Globe, Cpu, TerminalSquare, Fingerprint, LayoutGrid, Check } from "lucide-react"
import { cn } from "@/lib/utils"

interface Step {
  id: number
  icon: React.ReactNode
  title: string
  subtitle: string
  body: string
  visual: React.ReactNode
}

function GlobeVisual() {
  return (
    <div className="relative flex h-full items-center justify-center overflow-hidden">
      {/* Wireframe globe sketch */}
      <svg viewBox="0 0 200 200" className="h-40 w-40 text-primary opacity-60" fill="none">
        <ellipse cx="100" cy="100" rx="80" ry="80" stroke="currentColor" strokeWidth="0.8" strokeOpacity="0.4" />
        <ellipse cx="100" cy="100" rx="80" ry="28" stroke="currentColor" strokeWidth="0.8" strokeOpacity="0.4" />
        <ellipse cx="100" cy="100" rx="42" ry="80" stroke="currentColor" strokeWidth="0.8" strokeOpacity="0.4" />
        <ellipse cx="100" cy="100" rx="80" ry="55" stroke="currentColor" strokeWidth="0.8" strokeOpacity="0.3" />
        <line x1="20" y1="100" x2="180" y2="100" stroke="currentColor" strokeWidth="0.7" strokeOpacity="0.3" />
        <line x1="100" y1="20" x2="100" y2="180" stroke="currentColor" strokeWidth="0.7" strokeOpacity="0.3" />
        {/* Node dots */}
        {[
          [130, 65], [155, 110], [80, 130], [55, 75], [118, 148],
        ].map(([cx, cy], i) => (
          <circle key={i} cx={cx} cy={cy} r="3.5" fill="currentColor" fillOpacity="0.9" />
        ))}
        {/* Arc connections */}
        <path d="M130 65 Q155 85 155 110" stroke="currentColor" strokeWidth="1" strokeOpacity="0.5" />
        <path d="M55 75 Q60 100 80 130" stroke="currentColor" strokeWidth="1" strokeOpacity="0.5" />
        <path d="M130 65 Q105 95 80 130" stroke="currentColor" strokeWidth="1" strokeOpacity="0.4" />
        {/* User node */}
        <circle cx="60" cy="110" r="5" fill="currentColor" />
        <circle cx="60" cy="110" r="9" stroke="currentColor" strokeWidth="1" strokeOpacity="0.3" />
        <path d="M60 110 Q90 90 130 65" stroke="currentColor" strokeWidth="1.2" strokeOpacity="0.7" />
        <path d="M60 110 Q100 125 155 110" stroke="currentColor" strokeWidth="1.2" strokeOpacity="0.6" />
      </svg>
      <div className="absolute bottom-2 left-0 right-0 text-center font-mono text-[10px] text-muted-foreground">
        24 active nodes — 5 connections
      </div>
    </div>
  )
}

function WASIVisual() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 px-4">
      <div className="grid w-full max-w-[220px] grid-cols-3 gap-2">
        {["net-srv", "db-proxy", "ml-infer", "img-proc", "key-val", "auth-svc"].map((label) => (
          <div
            key={label}
            className="flex h-14 flex-col items-center justify-center rounded-lg border border-border bg-secondary/50 gap-1"
          >
            <Cpu className="h-4 w-4 text-primary" />
            <span className="font-mono text-[9px] text-muted-foreground">{label}</span>
          </div>
        ))}
      </div>
      <div className="font-mono text-[10px] text-muted-foreground">
        .wasm modules — sandboxed by default
      </div>
    </div>
  )
}

function CodeVisual() {
  const lines = [
    { t: "info", c: "$ edgerun run hello.wasm" },
    { t: "sys",  c: "  Loading module..." },
    { t: "ok",   c: "  WASI env initialized" },
    { t: "out",  c: '  stdout: "Hello, edge!"' },
    { t: "ok",   c: "  Exit code: 0  (12ms)" },
  ]
  return (
    <div className="flex h-full flex-col justify-center px-4">
      <div className="rounded-lg border border-border bg-[var(--terminal-bg)] p-4 font-mono text-xs leading-relaxed">
        {lines.map((line, i) => (
          <div
            key={i}
            className={cn(
              i === 0 && "text-foreground",
              line.t === "sys" && "text-muted-foreground",
              line.t === "ok" && "text-primary",
              line.t === "out" && "text-[var(--status-warning)]"
            )}
          >
            {line.c}
          </div>
        ))}
      </div>
    </div>
  )
}

function FingerprintVisual() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3">
      <div className="relative flex h-20 w-20 items-center justify-center rounded-full border-2 border-primary/40 bg-primary/5">
        <Fingerprint className="h-10 w-10 text-primary" />
        <span className="absolute -right-1 -top-1 flex h-5 w-5 items-center justify-center rounded-full bg-primary">
          <Check className="h-3 w-3 text-primary-foreground" />
        </span>
      </div>
      <div className="text-center">
        <p className="font-mono text-xs text-muted-foreground">
          Passkey bound to device
        </p>
        <p className="mt-1 font-mono text-[10px] text-muted-foreground opacity-60">
          WebAuthn · No passwords stored
        </p>
      </div>
    </div>
  )
}

function AppsVisual() {
  const apps = [
    { name: "Terminal", price: "Free" },
    { name: "App Store", price: "Free" },
    { name: "DB Explorer", price: "$5/mo" },
    { name: "Git Sync", price: "$3/mo" },
  ]
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-4">
      {apps.map((app) => (
        <div key={app.name} className="flex w-full max-w-[220px] items-center justify-between rounded-lg border border-border bg-secondary/40 px-3 py-2">
          <div className="flex items-center gap-2">
            <LayoutGrid className="h-3.5 w-3.5 text-primary" />
            <span className="text-xs text-foreground">{app.name}</span>
          </div>
          <span className={cn(
            "rounded px-1.5 py-0.5 text-[10px] font-medium",
            app.price === "Free" ? "bg-primary/20 text-primary" : "bg-[var(--status-warning)]/20 text-[var(--status-warning)]"
          )}>
            {app.price}
          </span>
        </div>
      ))}
    </div>
  )
}

const STEPS: Step[] = [
  {
    id: 1,
    icon: <Globe className="h-5 w-5" />,
    title: "A living network globe",
    subtitle: "Every node, in real time",
    body: "The globe behind your desktop shows all online Edgerun nodes across the planet. Your node is highlighted — arcs trace active connections as data flows between peers. The network is alive.",
    visual: <GlobeVisual />,
  },
  {
    id: 2,
    icon: <Cpu className="h-5 w-5" />,
    title: "Run WASI apps anywhere",
    subtitle: "WebAssembly, distributed",
    body: "Each app you launch is a WASM module executed in a sandboxed WASI environment. Modules are portable — they run identically on your node, on a peer's machine, or on bare metal at the edge.",
    visual: <WASIVisual />,
  },
  {
    id: 3,
    icon: <TerminalSquare className="h-5 w-5" />,
    title: "Install signed apps",
    subtitle: "Catalog proofs first",
    body: "Use the App Store or Terminal to install cataloged apps. Packages are verified against signed catalog records before the node accepts them.",
    visual: <CodeVisual />,
  },
  {
    id: 4,
    icon: <Fingerprint className="h-5 w-5" />,
    title: "Fingerprint-secured sessions",
    subtitle: "No passwords. Ever.",
    body: "Your identity is a WebAuthn passkey bound to your device's secure enclave. Lock the desktop at any time — the globe keeps running, but your apps are frozen until you scan again.",
    visual: <FingerprintVisual />,
  },
  {
    id: 5,
    icon: <LayoutGrid className="h-5 w-5" />,
    title: "The App Store",
    subtitle: "Install, run, compose",
    body: "Browse the built-in app catalog from the dock or App Store window. Free apps launch instantly. Premium apps are billed per node-hour. Mix and compose apps via the Terminal.",
    visual: <AppsVisual />,
  },
]

interface HelpAppProps {
  onFinish?: () => void
}

export function HelpApp({ onFinish }: HelpAppProps) {
  const [step, setStep] = useState(0)
  const current = STEPS[step]
  const isLast = step === STEPS.length - 1

  return (
    <div className="flex h-full flex-col">
      {/* Top — visual area */}
      <div className="relative flex h-[200px] flex-shrink-0 items-center justify-center border-b border-border bg-gradient-to-b from-secondary/20 to-transparent">
        {current.visual}
        {/* Step indicator dots */}
        <div className="absolute bottom-3 left-0 right-0 flex justify-center gap-1.5">
          {STEPS.map((s, i) => (
            <button
              key={s.id}
              onClick={() => setStep(i)}
              className={cn(
                "h-1.5 rounded-full transition-all duration-300",
                i === step ? "w-5 bg-primary" : "w-1.5 bg-border"
              )}
              aria-label={`Go to step ${i + 1}`}
            />
          ))}
        </div>
      </div>

      {/* Bottom — text + nav */}
      <div className="flex flex-1 flex-col p-5">
        {/* Step icon + label */}
        <div className="flex items-center gap-2 text-primary">
          {current.icon}
          <span className="font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
            {step + 1} / {STEPS.length}
          </span>
        </div>

        <h2 className="mt-2 text-base font-semibold text-foreground text-balance">{current.title}</h2>
        <p className="text-[11px] font-mono text-primary uppercase tracking-widest">{current.subtitle}</p>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{current.body}</p>

        {/* Navigation */}
        <div className="mt-auto flex items-center justify-between pt-4">
          <button
            onClick={() => setStep((s) => Math.max(0, s - 1))}
            disabled={step === 0}
            className="flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground disabled:opacity-30"
          >
            <ChevronLeft className="h-4 w-4" />
            Back
          </button>

          {isLast ? (
            <button
              onClick={onFinish}
              className="flex items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
            >
              <Check className="h-3.5 w-3.5" />
              Get started
            </button>
          ) : (
            <button
              onClick={() => setStep((s) => Math.min(STEPS.length - 1, s + 1))}
              className="flex items-center gap-1 text-sm text-foreground transition-colors hover:text-primary"
            >
              Next
              <ChevronRight className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
