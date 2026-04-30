"use client"

import { useState, useEffect } from "react"
import { cn } from "@/lib/utils"
import { EdgerunLogo } from "./edgerun-logo"
import { Globe } from "./globe"
import type { AuthState, NodeProvisionInput } from "@/hooks/use-auth"

interface AuthOverlayProps {
  authState: AuthState
  username: string
  isLoading: boolean
  error: string | null
  hasRegistered: () => boolean
  webAuthnAvailable: boolean
  onRegister: (name: string, nodeProvision?: NodeProvisionInput) => Promise<boolean>
  onAuthenticate: () => Promise<boolean>
  onClearError: () => void
}

type Screen = "welcome" | "register" | "lock"

// Fingerprint SVG icon
function FingerprintIcon({ className, scanning }: { className?: string; scanning?: boolean }) {
  return (
    <svg
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden="true"
    >
      {/* Outer ring */}
      <path
        d="M32 6C17.64 6 6 17.64 6 32"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        className={scanning ? "animate-[spin_2s_linear_infinite]" : ""}
        style={{ transformOrigin: "32px 32px" }}
        strokeOpacity={scanning ? 1 : 0.4}
      />
      <path d="M6 32C6 46.36 17.64 58 32 58C46.36 58 58 46.36 58 32C58 17.64 46.36 6 32 6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeOpacity="0.4" />

      {/* Ridge lines — fingerprint pattern */}
      <path d="M32 14C22.06 14 14 22.06 14 32" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeOpacity="0.6" />
      <path d="M50 32C50 22.06 41.94 14 32 14" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeOpacity="0.6" />
      <path d="M32 50C41.94 50 50 41.94 50 32" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeOpacity="0.5" />
      <path d="M14 32C14 41.94 22.06 50 32 50" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeOpacity="0.5" />

      <path d="M32 20C25.37 20 20 25.37 20 32" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeOpacity="0.7" />
      <path d="M44 32C44 25.37 38.63 20 32 20" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeOpacity="0.7" />
      <path d="M32 44C38.63 44 44 38.63 44 32" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeOpacity="0.6" />
      <path d="M20 32C20 38.63 25.37 44 32 44" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeOpacity="0.6" />

      {/* Center arc */}
      <path d="M27 32C27 29.24 29.24 27 32 27C34.76 27 37 29.24 37 32C37 34.76 34.76 37 32 37" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeOpacity="0.9" />

      {/* Center dot */}
      <circle cx="32" cy="32" r="2" fill="currentColor" />
    </svg>
  )
}

export function AuthOverlay({
  authState,
  username,
  isLoading,
  error,
  hasRegistered,
  webAuthnAvailable,
  onRegister,
  onAuthenticate,
  onClearError,
}: AuthOverlayProps) {
  const [screen, setScreen] = useState<Screen>("welcome")
  const [nameInput, setNameInput] = useState("")
  const [nodeIdInput, setNodeIdInput] = useState("")
  const [nodeTargetInput, setNodeTargetInput] = useState("127.0.0.1:35630")
  const [pairingPinInput, setPairingPinInput] = useState("")
  const [passphraseInput, setPassphraseInput] = useState("")
  const [scanState, setScanState] = useState<"idle" | "scanning" | "success" | "fail">("idle")

  const isLocalTarget = /^((localhost)|(127\.0\.0\.1)|(\[?::1\]?))(?::[0-9]{1,5})?$/.test(
    nodeTargetInput.trim(),
  )

  // Decide initial screen
  useEffect(() => {
    if (authState === "locked") {
      setScreen("lock")
    } else if (hasRegistered()) {
      setScreen("lock")
    } else {
      setScreen("welcome")
    }
  }, [authState, hasRegistered])

  // Map loading state to scan animation
  useEffect(() => {
    if (isLoading) setScanState("scanning")
    else if (error) setScanState("fail")
    else if (scanState === "scanning") setScanState("success")
  }, [isLoading, error]) // eslint-disable-line react-hooks/exhaustive-deps

  const handleFingerprint = async () => {
    onClearError()
    setScanState("scanning")
    const ok = await onAuthenticate()
    setScanState(ok ? "success" : "fail")
  }

  const handleRegister = async () => {
    if (!nameInput.trim()) return
    onClearError()
    const nodeProvision: NodeProvisionInput | undefined = webAuthnAvailable
      ? {
          nodeId: nodeIdInput,
          nodeTarget: nodeTargetInput,
          pairingPin: pairingPinInput,
          passphrase: passphraseInput,
        }
      : undefined

    setScanState("scanning")
    const ok = await onRegister(nameInput.trim(), nodeProvision)
    setScanState(ok ? "success" : "fail")
  }

  const canRegister = nameInput.trim().length > 0 && (!webAuthnAvailable || (
    nodeIdInput.trim().length > 0 &&
    (isLocalTarget || pairingPinInput.trim().length > 0) &&
    (isLocalTarget || passphraseInput.length >= 8)
  ))

  const scanColor = {
    idle: "text-muted-foreground",
    scanning: "text-primary",
    success: "text-primary",
    fail: "text-destructive",
  }[scanState]

  return (
    <div className="fixed inset-0 z-[100] flex overflow-hidden">
      {/* Globe background — full screen */}
      <div className="absolute inset-0">
        <Globe nodeCount={28} className="h-full w-full" />
        {/* Vignette overlay */}
        <div className="auth-vignette absolute inset-0" />
      </div>

      {/* Auth card — centered */}
      <div className="relative z-10 m-auto flex w-[360px] flex-col rounded-2xl border border-border bg-[var(--window-bg)]/90 backdrop-blur-xl shadow-2xl shadow-black/60">
        {/* Header */}
        <div className="flex items-center justify-center border-b border-border px-6 py-5">
          <EdgerunLogo variant="full" size="md" className="text-primary" />
        </div>

        <div className="flex flex-col items-center px-8 py-8 gap-6">
          {/* Welcome screen */}
          {screen === "welcome" && (
            <>
              <div className="text-center">
                <h2 className="text-lg font-semibold text-foreground">Welcome to Edgerun</h2>
                <p className="mt-1.5 text-sm text-muted-foreground leading-relaxed">
                  Secure your session with your device fingerprint. No passwords, no accounts.
                </p>
              </div>
              <button
                onClick={() => setScreen("register")}
                className="w-full rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 active:scale-[0.98]"
              >
                Get started
              </button>
              { (
                <button
                  onClick={() => setScreen("lock")}
                  className="text-xs text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
                >
                  Already have an account
                </button>
              )}
            </>
          )}

          {/* Register screen */}
          {screen === "register" && (
            <>
              <div className="text-center">
                <h2 className="text-base font-semibold text-foreground">Create your identity</h2>
                <p className="mt-1 text-xs text-muted-foreground">
                  Choose a handle, then scan your fingerprint to bind it.
                </p>
              </div>

              <input
                type="text"
                value={nameInput}
                onChange={(e) => setNameInput(e.target.value)}
                placeholder="Handle (e.g. ghost_node)"
                maxLength={24}
                className="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                onKeyDown={(e) => e.key === "Enter" && handleRegister()}
              />

              {webAuthnAvailable && (
                <>
                  <input
                    type="text"
                    value={nodeIdInput}
                    onChange={(e) => setNodeIdInput(e.target.value)}
                    placeholder="Node ID"
                    className="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                    onKeyDown={(e) => e.key === "Enter" && handleRegister()}
                  />

                  <input
                    type="text"
                    value={nodeTargetInput}
                    onChange={(e) => setNodeTargetInput(e.target.value)}
                    placeholder="Node target (e.g. 127.0.0.1:35630)"
                    className="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                    onKeyDown={(e) => e.key === "Enter" && handleRegister()}
                  />

                  <input
                    type="text"
                    value={pairingPinInput}
                    onChange={(e) => setPairingPinInput(e.target.value)}
                    placeholder="Pairing PIN"
                    className="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                    onKeyDown={(e) => e.key === "Enter" && handleRegister()}
                  />

                  <input
                    type="password"
                    value={passphraseInput}
                    onChange={(e) => setPassphraseInput(e.target.value)}
                    placeholder="Passphrase (min 8 chars)"
                    minLength={8}
                    className="w-full rounded-lg border border-border bg-secondary/50 px-3 py-2 font-mono text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                    onKeyDown={(e) => e.key === "Enter" && handleRegister()}
                  />
                </>
              )}

              {/* Fingerprint button */}
              <button
                onClick={handleRegister}
                disabled={isLoading || !canRegister}
                className={cn(
                  "group flex flex-col items-center gap-3 rounded-xl border p-5 transition-all w-full",
                  "border-border bg-secondary/30 hover:border-primary/50 hover:bg-primary/5",
                  "disabled:opacity-40 disabled:cursor-not-allowed",
                  scanState === "scanning" && "border-primary/60 bg-primary/5"
                )}
                aria-label="Scan fingerprint to register"
              >
                <FingerprintIcon
                  className={cn("h-12 w-12 transition-colors", scanColor)}
                  scanning={scanState === "scanning"}
                />
                <span className="text-xs font-mono text-muted-foreground">
                  {scanState === "scanning" ? "Scanning..." : scanState === "success" ? "Registered" : scanState === "fail" ? "Try again" : "Touch to register"}
                </span>
              </button>

              {error && (
                <p className="text-xs text-destructive text-center leading-relaxed">{error}</p>
              )}

              {!webAuthnAvailable && (
                <p className="text-[11px] text-muted-foreground text-center">
                  Biometrics not available on this device. Demo mode will be used.
                </p>
              )}

              <button
                onClick={() => { setScreen("welcome"); onClearError(); setScanState("idle") }}
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Back
              </button>
            </>
          )}

          {/* Lock screen */}
          {screen === "lock" && (
            <>
              <div className="text-center">
                {username ? (
                  <>
                    <p className="text-xs text-muted-foreground uppercase tracking-widest">Locked</p>
                    <h2 className="mt-1 font-mono text-lg font-semibold text-foreground">{username}</h2>
                  </>
                ) : (
                  <h2 className="text-base font-semibold text-foreground">Unlock session</h2>
                )}
                <p className="mt-1 text-xs text-muted-foreground">
                  {authState === "locked" ? "Your session is locked." : "Welcome back."} Verify your identity to continue.
                </p>
              </div>

              {/* Fingerprint button */}
              <button
                onClick={handleFingerprint}
                disabled={isLoading}
                className={cn(
                  "group flex flex-col items-center gap-3 rounded-xl border p-6 transition-all",
                  "border-border bg-secondary/30 hover:border-primary/50 hover:bg-primary/5",
                  "disabled:opacity-40 disabled:cursor-not-allowed",
                  scanState === "scanning" && "border-primary/60 bg-primary/5"
                )}
                aria-label="Scan fingerprint to unlock"
              >
                <FingerprintIcon
                  className={cn("h-14 w-14 transition-colors", scanColor)}
                  scanning={scanState === "scanning"}
                />
                <span className="text-xs font-mono text-muted-foreground">
                  {scanState === "scanning"
                    ? "Verifying..."
                    : scanState === "success"
                    ? "Unlocked"
                    : scanState === "fail"
                    ? "Failed — tap to retry"
                    : "Touch to unlock"}
                </span>
              </button>

              {error && (
                <p className="text-xs text-destructive text-center">{error}</p>
              )}

              <button
                onClick={() => { setScreen("welcome"); onClearError(); setScanState("idle") }}
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Use a different account
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
