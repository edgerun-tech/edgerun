"use client"

import { useState } from "react"
import { useStore } from "@nanostores/react"
import { Copy, HardDrive, IdCard, KeyRound, Lock, Mail, Network, ShieldCheck } from "lucide-react"
import { useAuth, type UnlockedProfileContainer } from "@/hooks/use-auth"
import { AppHeader } from "@/components/os/app-chrome"
import { browserAppInstallStore } from "@/platform/runtime/browser-app-install-store"
import { localCapabilityGrantsStore } from "@/stores/local-capability-grants-store"

import { shortHex } from "@/lib/format"
function shortId(value: string) {
  if (!value) return "missing"
  return shortHex(value, 14, 6)
}

function contactCard(handle: string, publicKey: string) {
  return [
    "Edgerun contact",
    `Name: ${handle}`,
    `Public encryption key: ${publicKey}`,
  ].join("\n")
}

function emailContact(handle: string, publicKey: string) {
  const subject = encodeURIComponent(`${handle}'s Edgerun contact`)
  const body = encodeURIComponent(contactCard(handle, publicKey))
  window.location.href = `mailto:?subject=${subject}&body=${body}`
}

export function IdentityApp() {
  const auth = useAuth()
  const appState = useStore(browserAppInstallStore)
  const capabilityGrants = useStore(localCapabilityGrantsStore)
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [password, setPassword] = useState("")
  const [deviceLabel, setDeviceLabel] = useState("")

  if (!profile) return null

  const cachedApps = appState.installed.size
  const capabilityGrantCount = Object.values(capabilityGrants).reduce((sum, grants) => sum + grants.length, 0)
  const passkeyBound = Boolean(profile.webAuthnBinding)

  async function copyContact() {
    if (!profile) return
    await navigator.clipboard.writeText(contactCard(profile.handle, profile.ownerEncryption.publicKeyRawBase64))
  }

  async function copyPublicKey() {
    if (!profile) return
    await navigator.clipboard.writeText(profile.ownerEncryption.publicKeyRawBase64)
  }

  async function copyBrowserNode() {
    if (!profile) return
    await navigator.clipboard.writeText(profile.browserNode.identityIdHex)
  }

  async function addDeviceIdentity() {
    await auth.createNode(deviceLabel, password)
    setDeviceLabel("")
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background">
      <AppHeader title="Identity" icon={<IdCard className="h-4 w-4" />}>
        Your Trust Container, browser node, passkey status, and public contact card
      </AppHeader>
      <div className="min-h-0 flex-1 overflow-auto p-4 sm:p-5">
        <section className="rounded-lg border border-border bg-card p-5">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start">
            <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-full bg-primary/12 text-lg font-semibold text-primary">
              {profile.handle.slice(0, 2).toUpperCase()}
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-lg font-semibold text-foreground">{profile.handle}</div>
              <div className="mt-1 text-sm text-muted-foreground">
                This browser is your Edgerun node. It can sign actions, run network apps, cache verified packages, and seal app keys locally.
              </div>
              <div className="mt-4 grid gap-2 sm:grid-cols-3">
                <button onClick={copyContact} className="flex items-center justify-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:opacity-90">
                  <Copy className="h-4 w-4" />
                  Copy contact
                </button>
                <button onClick={() => emailContact(profile.handle, profile.ownerEncryption.publicKeyRawBase64)} className="flex items-center justify-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm font-medium text-foreground hover:bg-secondary">
                  <Mail className="h-4 w-4" />
                  Send by email
                </button>
                <button onClick={copyBrowserNode} className="flex items-center justify-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm font-medium text-foreground hover:bg-secondary">
                  <Network className="h-4 w-4" />
                  Copy node id
                </button>
              </div>
            </div>
          </div>
        </section>

        <section className="mt-4 grid gap-3 sm:grid-cols-4">
          <div className="rounded-lg border border-border bg-card p-4">
            <ShieldCheck className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Owner identity</div>
            <div className="mt-1 font-mono text-xs text-foreground">{shortId(profile.owner.identityIdHex)}</div>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <Network className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Browser node</div>
            <div className="mt-1 font-mono text-xs text-foreground">{shortId(profile.browserNode.identityIdHex)}</div>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <KeyRound className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Passkey</div>
            <div className="mt-1 text-sm font-semibold text-foreground">{passkeyBound ? "Bound" : "Not bound"}</div>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <HardDrive className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Cached apps</div>
            <div className="mt-1 text-lg font-semibold text-foreground">{cachedApps}</div>
          </div>
        </section>

        <section className="mt-4 rounded-lg border border-border bg-card p-4">
          <div className="mb-3 flex items-center gap-2">
            <Lock className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-semibold text-foreground">Trust Container</h2>
          </div>
          <p className="text-xs leading-5 text-muted-foreground">
            This browser stores your profile as one AES-GCM sealed Trust Container. It holds your owner identity, browser-node identity, messaging keys, contacts, device records, app OAuth secrets, and local package/cache proofs. Trust Manager reads this same state as the proof dashboard.
          </p>
          <div className="mt-3 grid gap-2 sm:grid-cols-4">
            <div className="rounded-md border border-border bg-background/60 px-3 py-2">
              <div className="text-[11px] text-muted-foreground">Saved app keys</div>
              <div className="mt-1 text-sm font-semibold text-foreground">{profile.appSecrets.length}</div>
            </div>
            <div className="rounded-md border border-border bg-background/60 px-3 py-2">
              <div className="text-[11px] text-muted-foreground">Sealed messages/data</div>
              <div className="mt-1 text-sm font-semibold text-foreground">{profile.sealedContainers.length}</div>
            </div>
            <div className="rounded-md border border-border bg-background/60 px-3 py-2">
              <div className="text-[11px] text-muted-foreground">Capability grants</div>
              <div className="mt-1 text-sm font-semibold text-foreground">{capabilityGrantCount}</div>
            </div>
            <div className="rounded-md border border-border bg-background/60 px-3 py-2">
              <div className="text-[11px] text-muted-foreground">Profile events</div>
              <div className="mt-1 text-sm font-semibold text-foreground">{profile.eventLog.length}</div>
            </div>
          </div>
        </section>

        <section className="mt-4 rounded-lg border border-primary/20 bg-primary/5 p-4">
          <div className="text-sm font-semibold text-foreground">How this fits together</div>
          <div className="mt-2 grid gap-2 text-xs leading-5 text-muted-foreground sm:grid-cols-3">
            <div className="rounded-md border border-border bg-background/60 p-3">
              <div className="font-medium text-foreground">Identity signs</div>
              <div className="mt-1">Passkey/profile unlock lets the browser node sign run, cache, message, and payment intents.</div>
            </div>
            <div className="rounded-md border border-border bg-background/60 p-3">
              <div className="font-medium text-foreground">Apps run by hash</div>
              <div className="mt-1">SDK packages live in network storage. Local cache is optional and verifiable.</div>
            </div>
            <div className="rounded-md border border-border bg-background/60 p-3">
              <div className="font-medium text-foreground">Trust Manager proves</div>
              <div className="mt-1">Inspect package hashes, capability grants, browser-node routes, and runtime audit events.</div>
            </div>
          </div>
        </section>

        <details className="mt-4 rounded-lg border border-border bg-card">
          <summary className="cursor-pointer px-4 py-3 text-sm font-medium text-foreground">Advanced key details</summary>
          <div className="grid gap-3 border-t border-border p-4 text-xs">
            <div className="grid gap-1">
              <span className="text-muted-foreground">Public encryption key</span>
              <span className="break-all font-mono text-foreground">{profile.ownerEncryption.publicKeyRawBase64}</span>
            </div>
            <button onClick={copyPublicKey} className="inline-flex w-fit items-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-1.5 text-xs text-foreground hover:bg-secondary">
              <Copy className="h-3.5 w-3.5" />
              Copy public key
            </button>
            <div className="grid gap-2 border-t border-border pt-4">
              <label className="grid gap-1 text-muted-foreground">
                Profile password
                <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" placeholder="Required to add a device identity" className="rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus:border-primary" />
              </label>
              <input value={deviceLabel} onChange={(event) => setDeviceLabel(event.target.value)} placeholder="Device label" className="rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary" />
              <button disabled={!password || auth.isLoading} onClick={addDeviceIdentity} className="w-fit rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm text-foreground hover:bg-secondary disabled:opacity-45">
                Add device identity
              </button>
            </div>
          </div>
        </details>
      </div>
    </div>
  )
}
