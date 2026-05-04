"use client"

import * as React from "react"
import {
  AlertTriangle,
  CheckCircle2,
  Cloud,
  Copy,
  ExternalLink,
  Fingerprint,
  FileSignature,
  Github,
  KeyRound,
  Lock,
  RefreshCw,
  ShieldCheck,
  Trash2,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

type WorkspaceTab = "root" | "identity" | "connectors" | "audit"
type TrustRootState = "missing" | "local-draft" | "active"
type ConnectorState = "not-connected" | "ready-to-connect" | "connected" | "needs-review" | "revoked"

type AuditEntry = {
  id: string
  time: string
  title: string
  detail: string
  tone: "ok" | "warn" | "danger"
}

type Connector = {
  id: "github" | "cloudflare" | "google"
  name: string
  account: string
  purpose: string
  authority: string
  scopes: string[]
  state: ConnectorState
  icon: LucideIcon
  connectUrl: string
  honestStatus: string
  nextStep: string
}

type SigningKey = {
  publicJwk: JsonWebKey
  privateJwk: JsonWebKey
  createdAt: string
  keyId: string
}

const ROOT_STORAGE_KEY = "edgerun.trust.root.v1"
const SIGNING_KEY_STORAGE_KEY = "edgerun.trust.signingKey.v1"

const initialConnectors: Connector[] = [
  {
    id: "github",
    name: "GitHub",
    account: "Not connected",
    purpose: "Repositories, issues, pull requests, releases, deployment metadata",
    authority: "cloud:github",
    scopes: ["repo.metadata", "issues.read", "pull_requests.read", "releases.sign"],
    state: "ready-to-connect",
    icon: Github,
    connectUrl: "/api/github/auth",
    honestStatus: "OAuth route required. UI can model the permission now, but real account access needs the callback wired.",
    nextStep: "Connect GitHub, then map repos into signed Edgerun capabilities.",
  },
  {
    id: "cloudflare",
    name: "Cloudflare",
    account: "Not connected",
    purpose: "DNS, Pages, Workers, tunnels, zones, API tokens",
    authority: "cloud:cloudflare",
    scopes: ["zones.read", "dns.write", "pages.deploy", "tunnels.read"],
    state: "ready-to-connect",
    icon: Cloud,
    connectUrl: "/api/cloudflare/auth",
    honestStatus: "Cloudflare OAuth/API-token bridge is not wired yet. Treat this as the intended trust contract.",
    nextStep: "Add a scoped Cloudflare token, verify zones, then sign allowed actions.",
  },
  {
    id: "google",
    name: "Google",
    account: "Google account",
    purpose: "Gmail, Drive, Calendar, account data surfaces",
    authority: "cloud:google",
    scopes: ["gmail.read", "gmail.send", "drive.metadata", "calendar.read"],
    state: "connected",
    icon: ShieldCheck,
    connectUrl: "/api/gmail/auth",
    honestStatus: "Existing Gmail path appears present. Trust Manager should show exactly which Edgerun app receives each capability.",
    nextStep: "Split Google into Gmail, Drive, and Calendar capabilities instead of one vague connection.",
  },
]

function now() {
  return new Date().toLocaleString()
}

function shortKey(key?: string) {
  if (!key) return "not set"
  return key.length > 18 ? `${key.slice(0, 10)}…${key.slice(-6)}` : key
}

async function sha256(input: string) {
  const data = new TextEncoder().encode(input)
  const hash = await crypto.subtle.digest("SHA-256", data)
  return Array.from(new Uint8Array(hash)).map((byte) => byte.toString(16).padStart(2, "0")).join("")
}

function toBase64Url(bytes: ArrayBuffer) {
  let binary = ""
  for (const byte of new Uint8Array(bytes)) binary += String.fromCharCode(byte)
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "")
}

function statusClass(status: TrustRootState | ConnectorState | AuditEntry["tone"]) {
  switch (status) {
    case "active":
    case "connected":
    case "ok":
      return "border-[var(--status-online)]/25 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "local-draft":
    case "ready-to-connect":
    case "needs-review":
    case "warn":
      return "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "missing":
    case "not-connected":
    case "revoked":
    case "danger":
      return "border-[var(--status-error)]/25 bg-[var(--status-error)]/10 text-[var(--status-error)]"
  }
}

function Badge({ value }: { value: string }) {
  return <span className={cn("rounded-md border px-2 py-0.5 text-[10px] font-semibold uppercase", statusClass(value as any))}>{value}</span>
}

function Panel({ title, subtitle, icon: Icon, children }: { title: string; subtitle?: string; icon: LucideIcon; children: React.ReactNode }) {
  return (
    <section className="rounded-2xl border border-border bg-card/80 shadow-sm">
      <header className="flex items-start gap-3 border-b border-border px-4 py-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary">
          <Icon className="h-4 w-4" />
        </div>
        <div className="min-w-0">
          <h3 className="text-sm font-semibold text-foreground">{title}</h3>
          {subtitle ? <p className="mt-0.5 text-xs text-muted-foreground">{subtitle}</p> : null}
        </div>
      </header>
      <div className="p-4">{children}</div>
    </section>
  )
}

function KeyValue({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-border/50 py-2 last:border-b-0">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="min-w-0 truncate text-right font-mono text-xs text-foreground">{value}</span>
    </div>
  )
}

export function TrustManagerWorkspace() {
  const [tab, setTab] = React.useState<WorkspaceTab>("root")
  const [rootName, setRootName] = React.useState("Ken Personal Root")
  const [rootState, setRootState] = React.useState<TrustRootState>("missing")
  const [rootFingerprint, setRootFingerprint] = React.useState("")
  const [signingKey, setSigningKey] = React.useState<SigningKey | null>(null)
  const [message, setMessage] = React.useState("Set up a trust root first. Everything else should be signed back to it.")
  const [signingPayload, setSigningPayload] = React.useState("Authorize edgerun.tech to manage selected cloud resources through scoped capabilities.")
  const [signature, setSignature] = React.useState("")
  const [connectors, setConnectors] = React.useState(initialConnectors)
  const [audit, setAudit] = React.useState<AuditEntry[]>([])

  React.useEffect(() => {
    try {
      const savedRoot = window.localStorage.getItem(ROOT_STORAGE_KEY)
      if (savedRoot) {
        const parsed = JSON.parse(savedRoot) as { name: string; fingerprint: string; state: TrustRootState }
        setRootName(parsed.name)
        setRootFingerprint(parsed.fingerprint)
        setRootState(parsed.state)
      }
      const savedKey = window.localStorage.getItem(SIGNING_KEY_STORAGE_KEY)
      if (savedKey) setSigningKey(JSON.parse(savedKey) as SigningKey)
    } catch {
      setMessage("Local trust state exists but could not be parsed. Reset may be required.")
    }
  }, [])

  function log(title: string, detail: string, tone: AuditEntry["tone"] = "ok") {
    setAudit((prev) => [{ id: `${Date.now()}-${Math.random()}`, time: now(), title, detail, tone }, ...prev].slice(0, 30))
  }

  async function copy(value: string, label: string) {
    await navigator.clipboard.writeText(value).catch(() => {})
    setMessage(`Copied ${label}`)
  }

  async function createTrustRoot() {
    const fingerprint = await sha256(`${rootName}:${Date.now()}:${crypto.randomUUID()}`)
    const record = { name: rootName, fingerprint, state: "local-draft" as TrustRootState, createdAt: now() }
    window.localStorage.setItem(ROOT_STORAGE_KEY, JSON.stringify(record))
    setRootFingerprint(fingerprint)
    setRootState("local-draft")
    setMessage("Local trust root draft created. This is enough for UX/dev; production should bind it to hardware-backed or threshold keys.")
    log("Trust root draft created", `${rootName} → ${fingerprint.slice(0, 16)}…`, "warn")
  }

  function activateTrustRoot() {
    if (!rootFingerprint) return
    const record = { name: rootName, fingerprint: rootFingerprint, state: "active" as TrustRootState, activatedAt: now() }
    window.localStorage.setItem(ROOT_STORAGE_KEY, JSON.stringify(record))
    setRootState("active")
    setMessage("Trust root marked active locally. Real activation should require recovery policy and at least one backup signer.")
    log("Trust root activated locally", rootFingerprint, "ok")
  }

  async function generateSigningKey() {
    const key = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"])
    const publicJwk = await crypto.subtle.exportKey("jwk", key.publicKey)
    const privateJwk = await crypto.subtle.exportKey("jwk", key.privateKey)
    const keyId = (await sha256(JSON.stringify(publicJwk))).slice(0, 32)
    const record: SigningKey = { publicJwk, privateJwk, createdAt: now(), keyId }
    window.localStorage.setItem(SIGNING_KEY_STORAGE_KEY, JSON.stringify(record))
    setSigningKey(record)
    setMessage("Browser signing key created. This is useful for demos and local dev, not a final production root-key storage model.")
    log("Signing key generated", `P-256 browser key ${keyId}`, "warn")
  }

  async function signPayload() {
    if (!signingKey) {
      setMessage("Generate a signing key first.")
      return
    }
    const privateKey = await crypto.subtle.importKey("jwk", signingKey.privateJwk, { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"])
    const digest = await sha256(signingPayload)
    const signed = await crypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, privateKey, new TextEncoder().encode(signingPayload))
    const envelope = {
      type: "edgerun.signed-intent.v1",
      root: rootFingerprint || "missing-root",
      keyId: signingKey.keyId,
      algorithm: "ECDSA-P256-SHA256",
      payloadDigest: digest,
      payload: signingPayload,
      signature: toBase64Url(signed),
      createdAt: new Date().toISOString(),
    }
    const encoded = JSON.stringify(envelope, null, 2)
    setSignature(encoded)
    setMessage("Intent signed locally. This is the shape connectors should require before mutating cloud resources.")
    log("Intent signed", digest, "ok")
  }

  function updateConnector(id: Connector["id"], patch: Partial<Connector>) {
    setConnectors((prev) => prev.map((connector) => connector.id === id ? { ...connector, ...patch } : connector))
  }

  function markConnectorConnected(connector: Connector) {
    updateConnector(connector.id, { state: "connected", account: `${connector.name} account`, honestStatus: "Marked connected locally. Backend OAuth/token verification must replace this placeholder state." })
    setMessage(`${connector.name} marked connected locally. Next step is real OAuth/API token verification.`)
    log(`${connector.name} connector marked connected`, connector.authority, "warn")
  }

  function revokeConnector(connector: Connector) {
    updateConnector(connector.id, { state: "revoked", account: "Revoked locally" })
    setMessage(`${connector.name} revoked locally. Real implementation must also revoke provider tokens.`)
    log(`${connector.name} revoked`, connector.authority, "danger")
  }

  function resetLocalTrust() {
    window.localStorage.removeItem(ROOT_STORAGE_KEY)
    window.localStorage.removeItem(SIGNING_KEY_STORAGE_KEY)
    setRootState("missing")
    setRootFingerprint("")
    setSigningKey(null)
    setSignature("")
    setMessage("Local trust root and signing key removed.")
    log("Local trust reset", "Removed browser-stored root and signing key", "danger")
  }

  const tabs: { id: WorkspaceTab; label: string; icon: LucideIcon }[] = [
    { id: "root", label: "Trust Root", icon: ShieldCheck },
    { id: "identity", label: "Sign", icon: FileSignature },
    { id: "connectors", label: "Cloud", icon: Cloud },
    { id: "audit", label: "Audit", icon: Lock },
  ]

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-[var(--window-header)]/70">
        <div className="border-b border-border p-4">
          <div className="flex items-center gap-3">
            <div className={cn("flex h-10 w-10 items-center justify-center rounded-xl border", statusClass(rootState))}>
              <Fingerprint className="h-5 w-5" />
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold">Trust Manager</div>
              <div className="text-[10px] text-muted-foreground">root → sign → connect</div>
            </div>
          </div>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {tabs.map((item) => {
            const Icon = item.icon
            return (
              <button key={item.id} onClick={() => setTab(item.id)} className={cn("flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs font-medium", tab === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}>
                <Icon className="h-3.5 w-3.5" />
                {item.label}
              </button>
            )
          })}
        </nav>
        <div className="border-t border-border p-3 text-[10px] text-muted-foreground">
          <div>Root: <span className="font-mono text-foreground">{rootState}</span></div>
          <div className="mt-1 truncate">fp: {shortKey(rootFingerprint)}</div>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-4">
          <div>
            <h2 className="text-sm font-semibold">{tabs.find((item) => item.id === tab)?.label}</h2>
            <p className="text-[11px] text-muted-foreground">Be explicit: who is trusted, what can be signed, and which cloud actions are allowed.</p>
          </div>
          <div className="flex items-center gap-2">
            <Badge value={rootState} />
            <button onClick={() => setMessage("Refreshed local trust view")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium hover:bg-secondary/80"><RefreshCw className="mr-1 inline h-3.5 w-3.5" />Refresh</button>
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-auto p-4">
          {tab === "root" ? (
            <div className="grid gap-4 xl:grid-cols-[1fr_360px]">
              <Panel title="Set up trust root" subtitle="This is the identity that future devices, apps, connectors, and cloud actions anchor back to." icon={ShieldCheck}>
                <div className="space-y-4">
                  <div>
                    <label className="mb-1 block text-xs font-medium text-muted-foreground">Root name</label>
                    <input value={rootName} onChange={(event) => setRootName(event.target.value)} className="h-9 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary/60" />
                  </div>
                  <div className="rounded-xl border border-border bg-background/70 p-3">
                    <KeyValue label="State" value={<Badge value={rootState} />} />
                    <KeyValue label="Fingerprint" value={rootFingerprint || "not created"} />
                    <KeyValue label="Storage" value="browser localStorage" />
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button onClick={createTrustRoot} className="rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90">Create local root draft</button>
                    <button onClick={activateTrustRoot} disabled={!rootFingerprint} className="rounded-md bg-secondary px-3 py-2 text-xs font-medium hover:bg-secondary/80 disabled:opacity-40">Mark active locally</button>
                    <button onClick={() => copy(rootFingerprint, "root fingerprint")} disabled={!rootFingerprint} className="rounded-md bg-secondary px-3 py-2 text-xs font-medium hover:bg-secondary/80 disabled:opacity-40"><Copy className="mr-1 inline h-3 w-3" />Copy fingerprint</button>
                  </div>
                </div>
              </Panel>

              <Panel title="Honest production requirements" subtitle="Local root is fine for onboarding; production needs stronger guarantees." icon={AlertTriangle}>
                <div className="space-y-3 text-xs leading-relaxed text-muted-foreground">
                  <div className="rounded-lg border border-[var(--status-warning)]/20 bg-[var(--status-warning)]/5 p-3">Browser localStorage is not a secure root-key vault. Use it only for local demos and UX flow.</div>
                  <div className="rounded-lg border border-border bg-background/70 p-3">Real root should support hardware keys, passkeys, TPM/Secure Enclave where available, and threshold recovery.</div>
                  <div className="rounded-lg border border-border bg-background/70 p-3">Every connector mutation should require a signed intent envelope, not hidden ambient app authority.</div>
                  <button onClick={resetLocalTrust} className="w-full rounded-md bg-[var(--status-error)]/10 px-3 py-2 text-xs font-medium text-[var(--status-error)] hover:bg-[var(--status-error)]/20"><Trash2 className="mr-1 inline h-3.5 w-3.5" />Reset local trust state</button>
                </div>
              </Panel>
            </div>
          ) : null}

          {tab === "identity" ? (
            <div className="grid gap-4 xl:grid-cols-[1fr_420px]">
              <Panel title="Identify and sign things" subtitle="Create a signed intent before any cloud connector performs a privileged action." icon={FileSignature}>
                <div className="space-y-3">
                  <div className="rounded-xl border border-border bg-background/70 p-3">
                    <KeyValue label="Key type" value="ECDSA P-256 browser key" />
                    <KeyValue label="Key id" value={signingKey?.keyId ?? "not generated"} />
                    <KeyValue label="Root" value={shortKey(rootFingerprint)} />
                  </div>
                  <textarea value={signingPayload} onChange={(event) => setSigningPayload(event.target.value)} className="h-32 w-full resize-none rounded-md border border-border bg-background p-3 font-mono text-xs outline-none focus:border-primary/60" />
                  <div className="flex flex-wrap gap-2">
                    <button onClick={generateSigningKey} className="rounded-md bg-secondary px-3 py-2 text-xs font-medium hover:bg-secondary/80"><KeyRound className="mr-1 inline h-3.5 w-3.5" />Generate signing key</button>
                    <button onClick={signPayload} className="rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90">Sign intent</button>
                    <button onClick={() => copy(JSON.stringify(signingKey?.publicJwk ?? {}, null, 2), "public key")} disabled={!signingKey} className="rounded-md bg-secondary px-3 py-2 text-xs font-medium hover:bg-secondary/80 disabled:opacity-40">Copy public key</button>
                  </div>
                </div>
              </Panel>

              <Panel title="Signed envelope" subtitle="This is the artifact connectors should verify before doing work." icon={Lock}>
                <pre className="max-h-[420px] overflow-auto rounded-xl border border-border bg-zinc-950 p-3 text-[10px] leading-relaxed text-zinc-100">{signature || "No signature yet."}</pre>
              </Panel>
            </div>
          ) : null}

          {tab === "connectors" ? (
            <div className="grid gap-4 xl:grid-cols-3">
              {connectors.map((connector) => {
                const Icon = connector.icon
                return (
                  <Panel key={connector.id} title={connector.name} subtitle={connector.purpose} icon={Icon}>
                    <div className="space-y-3">
                      <div className="flex items-center justify-between gap-2"><span className="text-xs text-muted-foreground">State</span><Badge value={connector.state} /></div>
                      <div className="rounded-lg border border-border bg-background/70 p-3 text-xs">
                        <KeyValue label="Account" value={connector.account} />
                        <KeyValue label="Authority" value={connector.authority} />
                      </div>
                      <div className="flex flex-wrap gap-1.5">{connector.scopes.map((scope) => <span key={scope} className="rounded-md border border-primary/20 bg-primary/10 px-2 py-1 text-[11px] text-primary">{scope}</span>)}</div>
                      <div className="rounded-lg border border-border bg-background/70 p-3 text-xs leading-relaxed text-muted-foreground">{connector.honestStatus}</div>
                      <div className="rounded-lg border border-border bg-background/70 p-3 text-xs leading-relaxed text-muted-foreground">Next: {connector.nextStep}</div>
                      <div className="flex flex-wrap gap-2">
                        <button onClick={() => markConnectorConnected(connector)} className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90">Mark connected</button>
                        <button onClick={() => copy(connector.connectUrl, `${connector.name} route`)} className="rounded-md bg-secondary px-3 py-1.5 text-xs font-medium hover:bg-secondary/80"><ExternalLink className="mr-1 inline h-3 w-3" />Route</button>
                        <button onClick={() => revokeConnector(connector)} className="rounded-md bg-[var(--status-error)]/10 px-3 py-1.5 text-xs font-medium text-[var(--status-error)] hover:bg-[var(--status-error)]/20">Revoke</button>
                      </div>
                    </div>
                  </Panel>
                )
              })}
            </div>
          ) : null}

          {tab === "audit" ? (
            <Panel title="Audit log" subtitle="Local trust actions performed in this session." icon={Lock}>
              <div className="space-y-2">
                {audit.length === 0 ? <div className="rounded-xl border border-border bg-background/70 p-4 text-sm text-muted-foreground">No local trust actions yet.</div> : null}
                {audit.map((entry) => (
                  <div key={entry.id} className="rounded-xl border border-border bg-background/70 p-3">
                    <div className="flex items-center justify-between gap-3"><div className="text-sm font-medium text-foreground">{entry.title}</div><Badge value={entry.tone} /></div>
                    <div className="mt-1 font-mono text-[10px] text-muted-foreground">{entry.time}</div>
                    <div className="mt-2 text-xs text-muted-foreground">{entry.detail}</div>
                  </div>
                ))}
              </div>
            </Panel>
          ) : null}
        </div>

        <footer className="flex h-11 shrink-0 items-center justify-between border-t border-border bg-[var(--window-header)]/40 px-4">
          <p className="min-w-0 truncate text-[11px] text-muted-foreground">{message}</p>
          <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
            <CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" /> explicit authority only
          </div>
        </footer>
      </main>
    </div>
  )
}
