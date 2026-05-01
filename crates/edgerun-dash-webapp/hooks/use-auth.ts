"use client"

import { useState, useCallback, useEffect } from "react"

export type AuthState = "guest" | "unauthenticated" | "authenticated" | "locked"

export type NodeProvisionInput = {
  nodeId: string
  nodeTarget: string
  pairingPin: string
  passphrase: string
}

export type StoredNodeRegistration = {
  nodeId: string
  nodeTarget: string
  username: string
  registeredAtIso: string
}

const STORAGE_KEY = "edgerun_credential_id"
const USERNAME_KEY = "edgerun_username"
const NODE_REGISTRATION_KEY = "edgerun_node_registration_v1"
const RP_ID = typeof window !== "undefined" ? window.location.hostname : "localhost"
const RP_NAME = "Edgerun"

function bufferToBase64(buffer: ArrayBuffer): string {
  return btoa(String.fromCharCode(...new Uint8Array(buffer)))
}

function base64ToBuffer(base64: string): ArrayBuffer {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
  return bytes.buffer
}

function isWebAuthnAvailable(): boolean {
  return (
    typeof window !== "undefined" &&
    !!window.PublicKeyCredential &&
    typeof navigator.credentials?.create === "function"
  )
}

function normalizeNodeTarget(rawTarget: string): string {
  const trimmed = rawTarget.trim()
  if (!trimmed) {
    return "127.0.0.1:35630"
  }

  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    try {
      const parsed = new URL(trimmed)
      const host = parsed.hostname.trim()
      if (!host) return "127.0.0.1:35630"
      const port = parsed.port || "35630"
      return host.includes(":") ? `[${host}]:${port}` : `${host}:${port}`
    } catch {
      return "127.0.0.1:35630"
    }
  }

  if (trimmed.startsWith("[")) {
    const closed = trimmed.endsWith("]") ? trimmed : `${trimmed}]`
    return closed.includes("]:") ? closed : `${closed}:35630`
  }
  if (!trimmed.includes(":")) {
    return `${trimmed}:35630`
  }
  if ((trimmed.match(/:/g) || []).length > 1) {
    return `[${trimmed}]:35630`
  }
  return trimmed
}

function isLocalNodeTarget(rawTarget: string): boolean {
  const normalized = normalizeNodeTarget(rawTarget).trim().toLowerCase()
  if (normalized.startsWith("[")) {
    const end = normalized.indexOf("]")
    const host = end > 1 ? normalized.slice(1, end) : normalized
    return host === "::1" || host === "0:0:0:0:0:0:0:1"
  }
  const colon = normalized.lastIndexOf(":")
  const host = colon >= 0 ? normalized.slice(0, colon) : normalized
  return host === "localhost" || host === "127.0.0.1" || host === "::1"
}

type NormalizedNodeProvisionInput = NodeProvisionInput & {
  skipChecks: boolean
}

function normalizeProvisionInput(raw: NodeProvisionInput): NormalizedNodeProvisionInput {
  const nodeId = raw.nodeId.trim()
  const nodeTarget = normalizeNodeTarget(raw.nodeTarget)
  const pairingPin = raw.pairingPin.trim()
  const passphrase = raw.passphrase.trim()
  const skipChecks = isLocalNodeTarget(raw.nodeTarget)

  if (!nodeId || !pairingPin || !passphrase || !nodeTarget) {
    if (!skipChecks) {
      throw new Error("Missing node provisioning fields.")
    }
    if (!nodeId || !nodeTarget) {
      throw new Error("Missing node provisioning fields.")
    }
  }

  if (!skipChecks && passphrase.length < 8) {
    throw new Error("Passphrase must be at least 8 characters.")
  }

  return {
    nodeId,
    nodeTarget,
    pairingPin: skipChecks ? "" : pairingPin,
    passphrase: skipChecks ? "" : passphrase,
    skipChecks,
  }
}

type ProvisionNodeResponse = {
  status?: string
  ok?: boolean
  error?: string
}

function parseJsonResponse<T>(body: string): T | null {
  if (!body) return null
  try {
    return JSON.parse(body) as T
  } catch {
    return null
  }
}

function isNodeProvisionOk(response: ProvisionNodeResponse | null): boolean {
  if (response?.ok === false) return false
  if (!response?.status) return true
  return response.status === "provisioning_accepted" || response.status === "ok"
}

function nodeProvisionError(response: ProvisionNodeResponse | null, body: string): string {
  if (response?.error) return response.error
  if (response?.status && response.status !== "provisioning_accepted" && response.status !== "ok") {
    return `Node provisioning rejected (${response.status})`
  }
  if (response && response.ok === false) return "Node provisioning rejected"
  return body || "Node provisioning failed"
}

function readStoredRegistration(): StoredNodeRegistration | null {
  if (typeof window === "undefined") return null
  const raw = localStorage.getItem(NODE_REGISTRATION_KEY)
  if (!raw) return null

  try {
    const parsed = JSON.parse(raw) as StoredNodeRegistration
    if (
      typeof parsed.nodeId === "string" &&
      typeof parsed.nodeTarget === "string" &&
      typeof parsed.username === "string" &&
      typeof parsed.registeredAtIso === "string"
    ) {
      return parsed
    }
  } catch {
    localStorage.removeItem(NODE_REGISTRATION_KEY)
  }
  return null
}

function persistRegistration(registration: StoredNodeRegistration) {
  localStorage.setItem(NODE_REGISTRATION_KEY, JSON.stringify(registration))
}

async function provisionNode(input: NodeProvisionInput): Promise<void> {
  const normalized = normalizeProvisionInput(input) as NormalizedNodeProvisionInput
  const response = await fetch("/api/node/provision", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      node_id: normalized.nodeId,
      pin: normalized.pairingPin,
      passphrase: normalized.passphrase,
      password: normalized.passphrase,
      skip_checks: normalized.skipChecks,
      target: normalized.nodeTarget,
    }),
  })

  const text = await response.text()

  if (!response.ok) {
    throw new Error(text || `Node provisioning request failed (${response.status})`)
  }

  const payload = parseJsonResponse<ProvisionNodeResponse>(text)
  if (!isNodeProvisionOk(payload)) {
    throw new Error(nodeProvisionError(payload, text))
  }
}

export function useAuth() {
  const [authState, setAuthState] = useState<AuthState>("guest")
  const [username, setUsername] = useState<string>("")
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [webAuthnAvailable, setWebAuthnAvailable] = useState(false)
  const [nodeRegistration, setNodeRegistration] = useState<StoredNodeRegistration | null>(null)

  useEffect(() => {
    setWebAuthnAvailable(isWebAuthnAvailable())
    const storedUsername = typeof window !== "undefined" ? localStorage.getItem(USERNAME_KEY) : null
    if (storedUsername) setUsername(storedUsername)
    const storedRegistration = readStoredRegistration()
    if (storedRegistration) setNodeRegistration(storedRegistration)
  }, [])

  const hasRegistered = useCallback((): boolean => {
    if (typeof window === "undefined") return false
    return !!localStorage.getItem(STORAGE_KEY)
  }, [])

  /** Register a new passkey (fingerprint) */
  const register = useCallback(
    async (name: string, nodeProvision?: NodeProvisionInput): Promise<boolean> => {
      setIsLoading(true)
      setError(null)

      if (!webAuthnAvailable) {
        // Fallback: simulate registration for unsupported browsers
        await new Promise((r) => setTimeout(r, 1200))
        localStorage.setItem(STORAGE_KEY, "fallback-credential")
        localStorage.setItem(USERNAME_KEY, name)
        setUsername(name)
        setAuthState("authenticated")
        setIsLoading(false)
        return true
      }

      try {
        const normalizedProvision = nodeProvision ? normalizeProvisionInput(nodeProvision) : null
        const challenge = new Uint8Array(32)
        crypto.getRandomValues(challenge)

        const userId = new Uint8Array(16)
        crypto.getRandomValues(userId)

        const credential = (await navigator.credentials.create({
          publicKey: {
            challenge,
            rp: { id: RP_ID, name: RP_NAME },
            user: {
              id: userId,
              name,
              displayName: name,
            },
            pubKeyCredParams: [
              { alg: -7, type: "public-key" },
              { alg: -257, type: "public-key" },
            ],
            authenticatorSelection: {
              authenticatorAttachment: "cross-platform",
              userVerification: "required",
              residentKey: "preferred",
            },
            timeout: 60000,
            attestation: "none",
          },
        })) as PublicKeyCredential | null

        if (!credential) throw new Error("No credential returned")

        const credId = bufferToBase64(credential.rawId)

        if (!normalizedProvision) {
          throw new Error("Node provisioning details required for biometric registration.")
        }

        if (normalizedProvision) {
          await provisionNode(normalizedProvision)
          persistRegistration({
            nodeId: normalizedProvision.nodeId,
            nodeTarget: normalizedProvision.nodeTarget,
            username: name,
            registeredAtIso: new Date().toISOString(),
          })
          setNodeRegistration(readStoredRegistration())
        }

        localStorage.setItem(STORAGE_KEY, credId)
        localStorage.setItem(USERNAME_KEY, name)
        setUsername(name)
        setAuthState("authenticated")
        setIsLoading(false)
        return true
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Registration failed"
        localStorage.removeItem(STORAGE_KEY)
        localStorage.removeItem(USERNAME_KEY)
        localStorage.removeItem(NODE_REGISTRATION_KEY)
        setNodeRegistration(null)
        setError(
          msg.includes("cancel") || msg.includes("abort")
            ? "Fingerprint scan cancelled."
            : msg
        )
        setIsLoading(false)
        return false
      }
    },
    [webAuthnAvailable],
  )

  /** Authenticate with an existing passkey */
  const authenticate = useCallback(async (): Promise<boolean> => {
    setIsLoading(true)
    setError(null)

    const credIdB64 = typeof window !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null
    if (!credIdB64) {
      // No credential — enter guest mode instead of error
      setAuthState("guest")
      setIsLoading(false)
      return true
    }

    if (!webAuthnAvailable || credIdB64 === "fallback-credential") {
      // Fallback: just grant access
      await new Promise((r) => setTimeout(r, 1200))
      setAuthState("authenticated")
      setIsLoading(false)
      return true
    }

    try {
      const challenge = new Uint8Array(32)
      crypto.getRandomValues(challenge)

      const credId = base64ToBuffer(credIdB64)

      await navigator.credentials.get({
        publicKey: {
          challenge,
          rpId: RP_ID,
          allowCredentials: [{ id: credId, type: "public-key" }],
          userVerification: "required",
          timeout: 60000,
        },
      })

      setAuthState("authenticated")
      setIsLoading(false)
      return true
    } catch (err) {
      const msg = "Authentication failed"
      setError(msg.includes("cancel") || msg.includes("abort") ? "Fingerprint scan cancelled." : msg)
      setIsLoading(false)
      return false
    }
  }, [webAuthnAvailable])

  /** Continue as guest — no identity setup required */
  const continueAsGuest = useCallback(() => {
    setAuthState("guest")
    setUsername("")
  }, [])

  /** Lock the session */
  const lock = useCallback(() => {
    setAuthState("locked")
  }, [])

  /** Sign out completely */
  const signOut = useCallback(() => {
    localStorage.removeItem(STORAGE_KEY)
    localStorage.removeItem(USERNAME_KEY)
    localStorage.removeItem(NODE_REGISTRATION_KEY)
    setNodeRegistration(null)
    setUsername("")
    setAuthState("unauthenticated")
  }, [])

  const clearError = useCallback(() => setError(null), [])

  return {
    authState,
    username,
    isLoading,
    error,
    webAuthnAvailable,
    nodeRegistration,
    hasRegistered,
    register,
    authenticate,
    continueAsGuest,
    lock,
    signOut,
    clearError,
  }
}
