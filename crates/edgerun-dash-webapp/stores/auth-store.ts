import { atom, computed } from "nanostores"
import type { AuthState, NodeProvisionInput, StoredNodeRegistration } from "@/hooks/use-auth"

export type { AuthState, NodeProvisionInput, StoredNodeRegistration }

const STORAGE_KEY = "edgerun_credential_id"
const USERNAME_KEY = "edgerun_username"
const NODE_REGISTRATION_KEY = "edgerun_node_registration_v1"
const RP_ID = typeof window !== "undefined" ? window.location.hostname : "localhost"

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
  if (!trimmed) return "127.0.0.1:35630"

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
  if (!trimmed.includes(":")) return `${trimmed}:35630`
  if ((trimmed.match(/:/g) || []).length > 1) return `[${trimmed}]:35630`
  return trimmed
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
    ) return parsed
  } catch {
    localStorage.removeItem(NODE_REGISTRATION_KEY)
  }
  return null
}

function persistRegistration(registration: StoredNodeRegistration) {
  localStorage.setItem(NODE_REGISTRATION_KEY, JSON.stringify(registration))
}

export interface AuthStore {
  authState: AuthState
  username: string
  isLoading: boolean
  error: string | null
  webAuthnAvailable: boolean
  nodeRegistration: StoredNodeRegistration | null
}

export const authStore = atom<AuthStore>({
  authState: "guest",
  username: "",
  isLoading: false,
  error: null,
  webAuthnAvailable: false,
  nodeRegistration: null,
})

export const isAuthenticatedStore = computed(authStore, (s) => s.authState === "authenticated")
export const isGuestStore = computed(authStore, (s) => s.authState === "guest")

if (typeof window !== "undefined") {
  const storedUsername = localStorage.getItem(USERNAME_KEY)
  const storedRegistration = readStoredRegistration()
  authStore.set({
    authState: "guest",
    username: storedUsername || "",
    isLoading: false,
    error: null,
    webAuthnAvailable: isWebAuthnAvailable(),
    nodeRegistration: storedRegistration,
  })
}

export function hasRegistered(): boolean {
  if (typeof window === "undefined") return false
  return !!localStorage.getItem(STORAGE_KEY)
}

export async function registerAuth(name: string, nodeProvision?: NodeProvisionInput): Promise<boolean> {
  const state = authStore.get()
  if (state.isLoading) return false

  authStore.set({ ...state, isLoading: true, error: null })

  if (!state.webAuthnAvailable) {
    await new Promise((r) => setTimeout(r, 1200))
    localStorage.setItem(STORAGE_KEY, "fallback-credential")
    localStorage.setItem(USERNAME_KEY, name)
    authStore.set({ ...authStore.get(), username: name, authState: "authenticated", isLoading: false })
    return true
  }

  try {
    const challenge = new Uint8Array(32)
    crypto.getRandomValues(challenge)
    const userId = new Uint8Array(16)
    crypto.getRandomValues(userId)

    const credential = (await navigator.credentials.create({
      publicKey: {
        challenge,
        rp: { id: RP_ID, name: "Edgerun" },
        user: { id: userId, name, displayName: name },
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
    if (!nodeProvision) throw new Error("Node provisioning details required.")

    const normalized = normalizeNodeTarget(nodeProvision.nodeTarget)
    await fetch("/api/node/provision", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        node_id: nodeProvision.nodeId,
        pin: nodeProvision.pairingPin,
        passphrase: nodeProvision.passphrase,
        target: normalized,
      }),
    })

    persistRegistration({
      nodeId: nodeProvision.nodeId,
      nodeTarget: normalized,
      username: name,
      registeredAtIso: new Date().toISOString(),
    })

    localStorage.setItem(STORAGE_KEY, credId)
    localStorage.setItem(USERNAME_KEY, name)
    authStore.set({
      ...authStore.get(),
      username: name,
      authState: "authenticated",
      isLoading: false,
      nodeRegistration: readStoredRegistration(),
    })
    return true
  } catch (err) {
    const msg = err instanceof Error ? err.message : "Registration failed"
    localStorage.removeItem(STORAGE_KEY)
    localStorage.removeItem(USERNAME_KEY)
    localStorage.removeItem(NODE_REGISTRATION_KEY)
    authStore.set({
      ...authStore.get(),
      nodeRegistration: null,
      error: msg.includes("cancel") || msg.includes("abort") ? "Fingerprint scan cancelled." : msg,
      isLoading: false,
    })
    return false
  }
}

export async function authenticateAuth(): Promise<boolean> {
  const state = authStore.get()
  if (state.isLoading) return false

  authStore.set({ ...state, isLoading: true, error: null })

  const credIdB64 = typeof window !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null
  if (!credIdB64) {
    authStore.set({ ...authStore.get(), authState: "guest", isLoading: false })
    return true
  }

  if (!state.webAuthnAvailable || credIdB64 === "fallback-credential") {
    await new Promise((r) => setTimeout(r, 1200))
    authStore.set({ ...authStore.get(), authState: "authenticated", isLoading: false })
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

    authStore.set({ ...authStore.get(), authState: "authenticated", isLoading: false })
    return true
  } catch {
    authStore.set({
      ...authStore.get(),
      error: "Authentication failed",
      isLoading: false,
    })
    return false
  }
}

export function continueAsGuest() {
  authStore.set({ ...authStore.get(), authState: "guest", username: "" })
}

export function lockAuth() {
  authStore.set({ ...authStore.get(), authState: "locked" })
}

export function signOutAuth() {
  localStorage.removeItem(STORAGE_KEY)
  localStorage.removeItem(USERNAME_KEY)
  localStorage.removeItem(NODE_REGISTRATION_KEY)
  authStore.set({
    ...authStore.get(),
    nodeRegistration: null,
    username: "",
    authState: "unauthenticated",
  })
}

export function clearAuthError() {
  authStore.set({ ...authStore.get(), error: null })
}
