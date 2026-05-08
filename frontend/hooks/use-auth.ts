"use client"

import { useStore } from "@nanostores/react"
import {
  authStore,
  isAuthenticatedStore,
  isGuestStore,
  hasRegistered,
  registerAuth,
  authenticateAuth,
  authenticateWithWebAuthn,
  continueAsGuest,
  lockAuth,
  signOutAuth,
  clearAuthError,
  exportProfileContainer,
  importProfileContainer,
  switchProfile,
  createNestedSealedContainer,
  createProfileNode,
  addContactToProfile,
  openLocalQueuedMessage,
  updateProfilePreferences,
  bindWebAuthnToProfile,
  saveGmailProfileSecret,
  removeGmailProfileSecret,
  type AuthState,
  type NodeProvisionInput,
  type StoredNodeRegistration,
  type UnlockedProfileContainer,
  type SealedNestedContainer,
  type ContactRecord,
  type RoutedSealedEnvelope,
  type ProfileSummary,
  type LocalQueuedMessage,
  type ProfilePreferences,
  type ProfileEvent,
  type GmailProfileSecret,
} from "@/stores/auth-store"

export type { AuthState, NodeProvisionInput, StoredNodeRegistration, UnlockedProfileContainer, SealedNestedContainer, ContactRecord, RoutedSealedEnvelope, ProfileSummary, LocalQueuedMessage, ProfilePreferences, ProfileEvent, GmailProfileSecret }

export function useAuth() {
  const store = useStore(authStore)

  return {
    authState: store.authState,
    username: store.username,
    isLoading: store.isLoading,
    error: store.error,
    webAuthnAvailable: store.webAuthnAvailable,
    nodeRegistration: store.nodeRegistration,
    unlockedProfile: store.unlockedProfile,
    sealedProfile: store.sealedProfile,
    profileSummaries: store.profileSummaries,
    activeProfileId: store.activeProfileId,
    localMessages: store.localMessages,
    hasRegistered,
    register: registerAuth,
    authenticate: authenticateAuth,
    authenticateWithWebAuthn,
    continueAsGuest,
    lock: lockAuth,
    signOut: signOutAuth,
    clearError: clearAuthError,
    exportProfile: exportProfileContainer,
    importProfile: importProfileContainer,
    switchProfile,
    createNode: createProfileNode,
    createSealedContainer: createNestedSealedContainer,
    addContact: addContactToProfile,
    openLocalMessage: openLocalQueuedMessage,
    updateProfilePreferences,
    bindWebAuthn: bindWebAuthnToProfile,
    saveGmailSecret: saveGmailProfileSecret,
    removeGmailSecret: removeGmailProfileSecret,
  }
}

export { isAuthenticatedStore, isGuestStore }
