"use client"

import { useStore } from "@nanostores/react"
import {
  authStore,
  isAuthenticatedStore,
  isGuestStore,
  hasRegistered,
  registerAuth,
  authenticateAuth,
  continueAsGuest,
  lockAuth,
  signOutAuth,
  clearAuthError,
  type AuthState,
  type NodeProvisionInput,
  type StoredNodeRegistration,
} from "@/stores/auth-store"

export type { AuthState, NodeProvisionInput, StoredNodeRegistration }

export function useAuth() {
  const store = useStore(authStore)

  return {
    authState: store.authState,
    username: store.username,
    isLoading: store.isLoading,
    error: store.error,
    webAuthnAvailable: store.webAuthnAvailable,
    nodeRegistration: store.nodeRegistration,
    hasRegistered,
    register: registerAuth,
    authenticate: authenticateAuth,
    continueAsGuest,
    lock: lockAuth,
    signOut: signOutAuth,
    clearError: clearAuthError,
  }
}

export { isAuthenticatedStore, isGuestStore }
