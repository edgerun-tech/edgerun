"use client"

import { useState } from "react"
import { Copy, IdCard, KeyRound, Mail, ShieldCheck, X } from "lucide-react"
import { useAuth, type UnlockedProfileContainer } from "@/hooks/use-auth"

type IdentityAppProps = {
  onClose: () => void
}

function shortId(value: string) {
  if (!value) return "missing"
  return `${value.slice(0, 14)}...${value.slice(-6)}`
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

export function IdentityApp({ onClose }: IdentityAppProps) {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [password, setPassword] = useState("")
  const [deviceLabel, setDeviceLabel] = useState("")

  if (!profile) return null

  async function copyContact() {
    if (!profile) return
    await navigator.clipboard.writeText(contactCard(profile.handle, profile.ownerEncryption.publicKeyRawBase64))
  }

  async function copyPublicKey() {
    if (!profile) return
    await navigator.clipboard.writeText(profile.ownerEncryption.publicKeyRawBase64)
  }

  async function addDeviceIdentity() {
    await auth.createNode(deviceLabel, password)
    setDeviceLabel("")
  }

  return (
    <div className="fixed left-1/2 top-1/2 z-40 flex h-[min(640px,calc(100vh-6rem))] w-[min(860px,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border border-border bg-background/96 shadow-2xl backdrop-blur-xl">
      <div className="flex h-14 shrink-0 items-center justify-between border-b border-border px-4 sm:px-5">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-md bg-primary/12 text-primary"><IdCard className="h-4 w-4" /></div>
          <div>
            <div className="text-sm font-semibold text-foreground">Identity</div>
            <div className="text-xs text-muted-foreground">Your local profile and public contact card</div>
          </div>
        </div>
        <button onClick={onClose} className="rounded-md p-2 text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Close Identity">
          <X className="h-4 w-4" />
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-4 sm:p-5">
        <section className="rounded-lg border border-border bg-card p-5">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start">
            <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-full bg-primary/12 text-lg font-semibold text-primary">
              {profile.handle.slice(0, 2).toUpperCase()}
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-lg font-semibold text-foreground">{profile.handle}</div>
              <div className="mt-1 text-sm text-muted-foreground">Ready to receive encrypted messages</div>
              <div className="mt-4 grid gap-2 sm:grid-cols-2">
                <button onClick={copyContact} className="flex items-center justify-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:opacity-90">
                  <Copy className="h-4 w-4" />
                  Copy contact
                </button>
                <button onClick={() => emailContact(profile.handle, profile.ownerEncryption.publicKeyRawBase64)} className="flex items-center justify-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm font-medium text-foreground hover:bg-secondary">
                  <Mail className="h-4 w-4" />
                  Send by email
                </button>
              </div>
            </div>
          </div>
        </section>

        <section className="mt-4 grid gap-3 sm:grid-cols-3">
          <div className="rounded-lg border border-border bg-card p-4">
            <ShieldCheck className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Identity</div>
            <div className="mt-1 font-mono text-xs text-foreground">{shortId(profile.owner.identityIdHex)}</div>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <IdCard className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Contacts</div>
            <div className="mt-1 text-lg font-semibold text-foreground">{Math.max(profile.contacts.length - 1, 0)}</div>
          </div>
          <div className="rounded-lg border border-border bg-card p-4">
            <KeyRound className="mb-3 h-4 w-4 text-primary" />
            <div className="text-xs text-muted-foreground">Devices</div>
            <div className="mt-1 text-lg font-semibold text-foreground">{profile.nodes.length}</div>
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
