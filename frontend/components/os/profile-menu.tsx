"use client"

import { useMemo, useState } from "react"
import { BookOpen, Check, Copy, KeyRound, LogOut, Mail, Settings, Share2, User, X } from "lucide-react"
import { Switch } from "@/components/ui/switch"
import { useAuth, type ProfilePreferences, type UnlockedProfileContainer } from "@/hooks/use-auth"

type Panel = "menu" | "share" | "events" | "settings" | "confirm-logout"

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

function shortHash(value: string) {
  return `${value.slice(0, 10)}...${value.slice(-6)}`
}

export function ProfileMenu() {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [open, setOpen] = useState(false)
  const [panel, setPanel] = useState<Panel>("menu")
  const [copied, setCopied] = useState(false)
  const [password, setPassword] = useState("")
  const [draft, setDraft] = useState<ProfilePreferences | null>(null)

  const preferences = profile?.profilePreferences
  const effectiveDraft = draft ?? preferences
  const contactText = useMemo(() => {
    if (!profile) return ""
    return contactCard(profile.handle, profile.ownerEncryption.publicKeyRawBase64)
  }, [profile])

  if (!profile || !preferences || !effectiveDraft) return null

  function showPanel(next: Panel) {
    setPanel(next)
    setCopied(false)
    if (next === "settings") setDraft(profile?.profilePreferences ?? null)
  }

  async function copyContact() {
    await navigator.clipboard.writeText(contactText)
    setCopied(true)
  }

  async function saveSettings() {
    if (!draft || !password) return
    const ok = await auth.updateProfilePreferences({ password, patch: draft })
    if (ok) {
      setPassword("")
      setDraft(null)
      setPanel("menu")
    }
  }

  async function bindPasskey() {
    if (!password) return
    const ok = await auth.bindWebAuthn(password)
    if (ok) setPassword("")
  }

  function closeMenu() {
    setOpen(false)
    setPanel("menu")
    setPassword("")
    setDraft(null)
  }

  return (
    <div className="fixed right-4 top-4 z-50 sm:right-5 sm:top-5">
      <button
        onClick={() => setOpen((value) => !value)}
        className="flex h-11 w-11 items-center justify-center rounded-full bg-transparent text-white transition hover:text-foreground sm:h-12 sm:w-12"
        aria-label="Open profile menu"
      >
        <User className="h-5 w-5 text-muted-foreground" />
      </button>

      {open && (
        <div className="absolute right-0 mt-3 max-h-[calc(100vh-5.5rem)] w-[calc(100vw-2rem)] max-w-[360px] overflow-hidden rounded-xl border border-border bg-background/96 shadow-xl backdrop-blur-xl">
          <div className="flex items-center justify-between border-b border-border px-4 py-3">
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold text-foreground">{profile.handle}</div>
              <div className="text-xs text-muted-foreground">Identity settings</div>
            </div>
            <button onClick={closeMenu} className="rounded-md p-1.5 text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Close profile menu">
              <X className="h-4 w-4" />
            </button>
          </div>

          {panel === "menu" && (
            <div className="p-2.5">
              <button onClick={() => showPanel("share")} className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm text-foreground hover:bg-secondary">
                <Share2 className="h-4 w-4 text-primary" />
                Share profile
              </button>
              <button onClick={() => showPanel("events")} className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm text-foreground hover:bg-secondary">
                <BookOpen className="h-4 w-4 text-primary" />
                Event log
              </button>
              <button onClick={() => showPanel("settings")} className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm text-foreground hover:bg-secondary">
                <Settings className="h-4 w-4 text-primary" />
                Profile settings
              </button>
              <div className="my-2 border-t border-border" />
              <button onClick={() => showPanel("confirm-logout")} className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm text-destructive hover:bg-destructive/10">
                <LogOut className="h-4 w-4" />
                Log out
              </button>
            </div>
          )}

          {panel === "share" && (
            <div className="grid gap-3 p-4">
              <div className="max-h-36 overflow-auto rounded-lg border border-border bg-secondary/35 p-3 font-mono text-[11px] leading-4 text-muted-foreground">
                {contactText}
              </div>
              <div className="grid grid-cols-2 gap-2">
                <button onClick={copyContact} className="flex items-center justify-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground">
                  {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                  {copied ? "Copied" : "Copy"}
                </button>
                <button onClick={() => emailContact(profile.handle, profile.ownerEncryption.publicKeyRawBase64)} className="flex items-center justify-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm text-foreground hover:bg-secondary">
                  <Mail className="h-4 w-4" />
                  Email
                </button>
              </div>
              <button onClick={() => showPanel("menu")} className="text-xs text-muted-foreground hover:text-foreground">Back</button>
            </div>
          )}

          {panel === "events" && (
            <div className="max-h-[min(420px,calc(100vh-10rem))] overflow-auto p-4">
              <div className="grid gap-2">
                {profile.eventLog.map((event) => (
                  <div key={event.eventHash} className="rounded-lg border border-border bg-card p-3">
                    <div className="flex items-center justify-between gap-3">
                      <div className="text-xs font-semibold text-foreground">{event.kind}</div>
                      <div className="font-mono text-[10px] text-muted-foreground">#{event.seq}</div>
                    </div>
                    <div className="mt-1 font-mono text-[10px] text-muted-foreground">{shortHash(event.eventHash)}</div>
                  </div>
                ))}
              </div>
              <button onClick={() => showPanel("menu")} className="mt-3 text-xs text-muted-foreground hover:text-foreground">Back</button>
            </div>
          )}

          {panel === "settings" && (
            <div className="max-h-[min(560px,calc(100vh-10rem))] overflow-auto p-4">
              <div className="grid gap-4">
              <div className="grid gap-3">
                <div className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-3 py-2">
                  <div>
                    <div className="text-sm text-foreground">Connect to network</div>
                    <div className="text-xs text-muted-foreground">Allow this identity to connect when networking is available.</div>
                  </div>
                  <Switch checked={effectiveDraft.connectToNetwork} onCheckedChange={(value) => setDraft({ ...effectiveDraft, connectToNetwork: value })} />
                </div>
                <div className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-3 py-2">
                  <div>
                    <div className="text-sm text-foreground">Share resources</div>
                    <div className="text-xs text-muted-foreground">Allow spare local resources to be shared.</div>
                  </div>
                  <Switch checked={effectiveDraft.shareResources} onCheckedChange={(value) => setDraft({ ...effectiveDraft, shareResources: value })} />
                </div>
              </div>
              <div className="rounded-lg border border-border bg-card px-3 py-3">
                <div className="flex items-start gap-3">
                  <KeyRound className="mt-0.5 h-4 w-4 text-primary" />
                  <div className="min-w-0 flex-1">
                    <div className="text-sm text-foreground">Passkey unlock</div>
                    <div className="text-xs text-muted-foreground">
                      {profile.webAuthnBinding ? "A passkey is bound on this browser." : "Bind a passkey for quicker unlock on this browser."}
                    </div>
                  </div>
                </div>
                <button disabled={!password || auth.isLoading} onClick={bindPasskey} className="mt-3 w-full rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm text-foreground hover:bg-secondary disabled:opacity-45">
                  {profile.webAuthnBinding ? "Rebind passkey" : "Bind passkey"}
                </button>
              </div>
              <input
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                type="password"
                placeholder="Profile password to save"
                className="rounded-md border border-border bg-secondary px-3 py-2 text-sm text-foreground outline-none focus:border-primary"
              />
              <div className="grid grid-cols-2 gap-2">
                <button onClick={() => showPanel("menu")} className="rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm text-foreground hover:bg-secondary">Cancel</button>
                <button disabled={!password || auth.isLoading} onClick={saveSettings} className="rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-45">
                  Save
                </button>
              </div>
              </div>
            </div>
          )}

          {panel === "confirm-logout" && (
            <div className="grid gap-3 p-4">
              <div>
                <div className="text-sm font-semibold text-foreground">Log out?</div>
                <div className="mt-1 text-xs text-muted-foreground">This locks the profile and removes usable key material from this session.</div>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <button onClick={() => showPanel("menu")} className="rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm text-foreground hover:bg-secondary">Cancel</button>
                <button onClick={() => auth.lock()} className="rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground">Log out</button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
