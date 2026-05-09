"use client"

import { useEffect, useRef, useState } from "react"
import { Download, KeyRound, LockKeyhole, Upload } from "lucide-react"
import { EdgerunLogo } from "./edgerun-logo"
import type { AuthState, NodeProvisionInput, ProfileSummary } from "@/hooks/use-auth"

interface AuthOverlayProps {
  authState: AuthState
  username: string
  isLoading: boolean
  error: string | null
  hasRegistered: () => boolean
  profileSummaries: ProfileSummary[]
  activeProfileId: string | null
  onRegister: (name: string, nodeProvision?: NodeProvisionInput) => Promise<boolean>
  onAuthenticate: (password?: string) => Promise<boolean>
  onAuthenticateWithWebAuthn: () => Promise<boolean>
  onBindWebAuthn: (password: string) => Promise<boolean>
  onSwitchProfile: (profileId: string) => void
  onClearError: () => void
  onExportProfile: () => string | null
  onImportProfile: (serialized: string) => Promise<boolean>
}

type Screen = "welcome" | "create" | "unlock" | "import"

export function AuthOverlay({
  authState,
  isLoading,
  error,
  hasRegistered,
  profileSummaries,
  activeProfileId,
  onRegister,
  onAuthenticate,
  onAuthenticateWithWebAuthn,
  onBindWebAuthn,
  onSwitchProfile,
  onClearError,
  onExportProfile,
  onImportProfile,
}: AuthOverlayProps) {
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [screen, setScreen] = useState<Screen>("welcome")
  const [handle, setHandle] = useState("")
  const [password, setPassword] = useState("")
  const [confirmPassword, setConfirmPassword] = useState("")
  const [bindPasskeyAfterCreate, setBindPasskeyAfterCreate] = useState(false)

  useEffect(() => {
    if (authState === "locked" || hasRegistered()) setScreen("unlock")
    else setScreen("welcome")
  }, [authState, hasRegistered])

  const canCreate = handle.trim().length >= 2 && password.length >= 8 && password === confirmPassword
  const canUnlock = password.length > 0
  const activeProfile = profileSummaries.find((profile) => profile.profileId === activeProfileId) ?? profileSummaries[0]
  const canUnlockWithPasskey = Boolean(activeProfile?.webAuthnUnlockAvailable)

  async function createProfile() {
    if (!canCreate) return
    onClearError()
    const profilePassword = password
    const ok = await onRegister(handle.trim(), { passphrase: profilePassword })
    if (ok && bindPasskeyAfterCreate) {
      await onBindWebAuthn(profilePassword)
    }
    if (ok) {
      setPassword("")
      setConfirmPassword("")
    }
  }

  async function unlockProfile() {
    if (!canUnlock) return
    onClearError()
    await onAuthenticate(password)
  }

  async function importProfile(file: File | undefined) {
    if (!file) return
    onClearError()
    const ok = await onImportProfile(await file.text())
    if (ok) {
      setPassword("")
      setScreen("unlock")
    }
  }

  function exportProfile() {
    const serialized = onExportProfile()
    if (!serialized) return
    const blob = new Blob([serialized], { type: "application/vnd.edgerun.profile+json" })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement("a")
    anchor.href = url
    anchor.download = "edgerun-profile.eusr-container.json"
    anchor.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="fixed inset-0 z-[100] flex overflow-hidden">
      <div className="absolute inset-0 bg-background" />

      <div className="edgerun-auth-panel relative z-10 m-auto flex flex-col overflow-hidden rounded-2xl border border-border bg-[var(--window-bg)]/94 shadow-2xl shadow-black/60 backdrop-blur-xl">
        <div className="flex items-center justify-center border-b border-border px-6 py-5 sm:py-6">
          <EdgerunLogo variant="full" size="md" className="text-primary" />
        </div>

        <div className="flex min-h-0 flex-col gap-5 overflow-auto px-5 py-6 sm:px-7 sm:py-7">
          {screen === "welcome" && (
            <>
              <div>
                <h2 className="text-lg font-semibold text-foreground">Set up your profile</h2>
                <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                  Create a local identity for this browser. You can lock it, switch profiles, and bring it back later with your password.
                </p>
              </div>
              <button onClick={() => setScreen("create")} className="flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-primary/90">
                <KeyRound className="h-4 w-4" />
                New profile
              </button>
              <button onClick={() => fileInputRef.current?.click()} className="flex h-11 w-full items-center justify-center gap-2 rounded-lg border border-border bg-secondary/50 px-4 text-sm font-medium text-foreground hover:bg-secondary">
                <Upload className="h-4 w-4" />
                Import profile
              </button>
            </>
          )}

          {screen === "create" && (
            <>
              <div>
                <h2 className="text-base font-semibold text-foreground">New profile</h2>
                <p className="mt-1 text-xs text-muted-foreground">
                  Choose a name and password. The keys stay sealed on this device.
                </p>
              </div>
              <input value={handle} onChange={(event) => setHandle(event.target.value)} placeholder="Handle" className="h-10 w-full rounded-lg border border-border bg-secondary/50 px-3 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20" />
              <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" placeholder="Profile password" className="h-10 w-full rounded-lg border border-border bg-secondary/50 px-3 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20" />
              <input value={confirmPassword} onChange={(event) => setConfirmPassword(event.target.value)} type="password" placeholder="Confirm password" className="h-10 w-full rounded-lg border border-border bg-secondary/50 px-3 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20" onKeyDown={(event) => event.key === "Enter" && createProfile()} />
              <label className="flex items-start gap-3 rounded-lg border border-border bg-secondary/35 px-3 py-2 text-xs text-muted-foreground">
                <input
                  type="checkbox"
                  checked={bindPasskeyAfterCreate}
                  onChange={(event) => setBindPasskeyAfterCreate(event.target.checked)}
                  className="mt-0.5"
                />
                <span>
                  <span className="block text-foreground">Bind passkey after creating</span>
                  <span className="block">Use fingerprint, Face ID, Windows Hello, or a security key for future unlocks.</span>
                </span>
              </label>
              <button disabled={!canCreate || isLoading} onClick={createProfile} className="flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 text-sm font-semibold text-primary-foreground disabled:cursor-not-allowed disabled:opacity-45">
                <LockKeyhole className="h-4 w-4" />
                {isLoading ? "Sealing..." : "Create and unlock"}
              </button>
              <button onClick={() => setScreen("welcome")} className="text-xs text-muted-foreground hover:text-foreground">Back</button>
            </>
          )}

          {screen === "unlock" && (
            <>
              <div>
                <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">Locked</p>
                <h2 className="mt-1 text-base font-semibold text-foreground">Open your profile</h2>
                <p className="mt-1 text-xs text-muted-foreground">
                  Select a profile and enter its password. A recent unlock can survive one refresh for about 30 seconds in this tab.
                </p>
              </div>
              {profileSummaries.length > 0 && (
                <select
                  value={activeProfileId ?? profileSummaries[0]?.profileId ?? ""}
                  onChange={(event) => {
                    setPassword("")
                    onSwitchProfile(event.target.value)
                  }}
                  className="h-10 w-full rounded-lg border border-border bg-secondary/50 px-3 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20"
                >
                  {profileSummaries.map((profile) => (
                    <option key={profile.profileId} value={profile.profileId}>
                      {profile.handle} · {profile.profileId.slice(0, 12)}
                    </option>
                  ))}
                </select>
              )}
              <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" placeholder="Profile password" className="h-10 w-full rounded-lg border border-border bg-secondary/50 px-3 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20" onKeyDown={(event) => event.key === "Enter" && unlockProfile()} />
              <button disabled={!canUnlock || isLoading} onClick={unlockProfile} className="flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 text-sm font-semibold text-primary-foreground disabled:cursor-not-allowed disabled:opacity-45">
                <LockKeyhole className="h-4 w-4" />
                {isLoading ? "Opening..." : "Unlock"}
              </button>
              {canUnlockWithPasskey && (
                <button disabled={isLoading} onClick={onAuthenticateWithWebAuthn} className="flex h-11 w-full items-center justify-center gap-2 rounded-lg border border-border bg-secondary/50 px-4 text-sm font-medium text-foreground hover:bg-secondary disabled:opacity-45">
                  <KeyRound className="h-4 w-4" />
                  Unlock with passkey
                </button>
              )}
              <div className="grid grid-cols-2 gap-2">
                <button onClick={exportProfile} className="flex items-center justify-center gap-2 rounded-lg border border-border bg-secondary/50 px-3 py-2 text-xs text-foreground hover:bg-secondary">
                  <Download className="h-3.5 w-3.5" />
                  Export
                </button>
                <button onClick={() => fileInputRef.current?.click()} className="flex items-center justify-center gap-2 rounded-lg border border-border bg-secondary/50 px-3 py-2 text-xs text-foreground hover:bg-secondary">
                  <Upload className="h-3.5 w-3.5" />
                  Import
                </button>
              </div>
              <button onClick={() => setScreen("create")} className="text-xs text-muted-foreground hover:text-foreground">Create another profile</button>
            </>
          )}

          {error && <p className="rounded-lg border border-destructive/25 bg-destructive/10 px-3 py-2 text-xs text-destructive">{error}</p>}
        </div>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".json,.eusr,.profile,application/json,application/vnd.edgerun.profile+json"
        className="hidden"
        onChange={(event) => importProfile(event.target.files?.[0])}
      />
    </div>
  )
}
