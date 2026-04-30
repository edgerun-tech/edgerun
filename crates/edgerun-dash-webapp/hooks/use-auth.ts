"use client"

import { useState, useCallback, useEffect } from "react"

export type AuthState = "unauthenticated" | "authenticated" | "locked"

const STORAGE_KEY = "edgerun_credential_id"
const USERNAME_KEY = "edgerun_username"
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

export function useAuth() {
  const [authState, setAuthState] = useState<AuthState>("unauthenticated")
  const [username, setUsername] = useState<string>("")
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [webAuthnAvailable, setWebAuthnAvailable] = useState(false)

  useEffect(() => {
    setWebAuthnAvailable(isWebAuthnAvailable())
    const storedUsername = typeof window !== "undefined" ? localStorage.getItem(USERNAME_KEY) : null
    if (storedUsername) setUsername(storedUsername)
  }, [])

  const hasRegistered = useCallback((): boolean => {
    if (typeof window === "undefined") return false
    return !!localStorage.getItem(STORAGE_KEY)
  }, [])

  /** Register a new passkey (fingerprint) */
  const register = useCallback(async (name: string): Promise<boolean> => {
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
      const challenge = new Uint8Array(32)
      crypto.getRandomValues(challenge)

      const userId = new Uint8Array(16)
      crypto.getRandomValues(userId)

      const credential = await navigator.credentials.create({
        publicKey: {
          challenge,
          rp: { id: RP_ID, name: RP_NAME },
          user: {
            id: userId,
            name,
            displayName: name,
          },
          pubKeyCredParams: [
            { alg: -7, type: "public-key" },   // ES256
            { alg: -257, type: "public-key" },  // RS256
          ],
          authenticatorSelection: {
            authenticatorAttachment: "platform",
            userVerification: "required",
            residentKey: "preferred",
          },
          timeout: 60000,
          attestation: "none",
        },
      }) as PublicKeyCredential | null

      if (!credential) throw new Error("No credential returned")

      const credId = bufferToBase64(credential.rawId)
      localStorage.setItem(STORAGE_KEY, credId)
      localStorage.setItem(USERNAME_KEY, name)
      setUsername(name)
      setAuthState("authenticated")
      setIsLoading(false)
      return true
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Registration failed"
      // User cancelled: treat as soft error
      setError(msg.includes("cancel") || msg.includes("abort") ? "Fingerprint scan cancelled." : msg)
      setIsLoading(false)
      return false
    }
  }, [webAuthnAvailable])

  /** Authenticate with an existing passkey */
  const authenticate = useCallback(async (): Promise<boolean> => {
    setIsLoading(true)
    setError(null)

    const credIdB64 = typeof window !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null
    if (!credIdB64) {
      setError("No registered credential found. Please sign up first.")
      setIsLoading(false)
      return false
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
      const msg = err instanceof Error ? err.message : "Authentication failed"
      setError(msg.includes("cancel") || msg.includes("abort") ? "Fingerprint scan cancelled." : msg)
      setIsLoading(false)
      return false
    }
  }, [webAuthnAvailable])

  /** Lock the session */
  const lock = useCallback(() => {
    setAuthState("locked")
  }, [])

  /** Sign out completely */
  const signOut = useCallback(() => {
    localStorage.removeItem(STORAGE_KEY)
    localStorage.removeItem(USERNAME_KEY)
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
    hasRegistered,
    register,
    authenticate,
    lock,
    signOut,
    clearError,
  }
}
