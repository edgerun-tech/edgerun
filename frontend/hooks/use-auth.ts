"use client"

import { useEffect, useRef } from "react"
import { useStore } from "@nanostores/react"
import {
  authStore,
  isAuthenticatedStore,
  isGuestStore,
  hasRegistered,
  registerAuth,
  authenticateAuth,
  authenticateWithWebAuthn,
  resumeSessionAuth,
  continueAsGuest,
  lockAuth,
  signOutAuth,
  clearAuthError,
  exportProfileContainer,
  importProfileContainer,
  switchProfile,
  createNestedSealedContainer,
  createProfileNode,
  publishBrowserNodeRelayRoute,
  addContactToProfile,
  saveContactToProfile,
  openLocalQueuedMessage,
  updateProfilePreferences,
  bindWebAuthnToProfile,
  saveGmailProfileSecret,
  removeGmailProfileSecret,
  saveOAuthProfileSecret,
  removeOAuthProfileSecret,
  type AuthState,
  type NodeProvisionInput,
  type StoredNodeRegistration,
  type UnlockedProfileContainer,
  type SealedNestedContainer,
  type ContactRecord,
  type RoutedSealedEnvelope,
  type ProfileSummary,
  type LocalQueuedMessage,
  type NodeRelayPublishResult,
  type ProfilePreferences,
  type ProfileEvent,
  type GmailProfileSecret,
  type GoogleDriveProfileSecret,
  type GitHubProfileSecret,
  type CloudflareProfileSecret,
  type OAuthProfileSecret,
} from "@/stores/auth-store"

export type { AuthState, NodeProvisionInput, StoredNodeRegistration, UnlockedProfileContainer, SealedNestedContainer, ContactRecord, RoutedSealedEnvelope, ProfileSummary, LocalQueuedMessage, NodeRelayPublishResult, ProfilePreferences, ProfileEvent, GmailProfileSecret, GoogleDriveProfileSecret, GitHubProfileSecret, CloudflareProfileSecret, OAuthProfileSecret }

export function useAuth() {
  const store = useStore(authStore)
  const resumeAttemptedRef = useRef(false)

  useEffect(() => {
    if (store.authState !== "locked" || resumeAttemptedRef.current) return
    resumeAttemptedRef.current = true
    void resumeSessionAuth()
  }, [store.authState])

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
    resumeSession: resumeSessionAuth,
    continueAsGuest,
    lock: lockAuth,
    signOut: signOutAuth,
    clearError: clearAuthError,
    exportProfile: exportProfileContainer,
    importProfile: importProfileContainer,
    switchProfile,
    createNode: createProfileNode,
    publishNodeRelayRoute: publishBrowserNodeRelayRoute,
    createSealedContainer: createNestedSealedContainer,
    addContact: addContactToProfile,
    saveContact: saveContactToProfile,
    openLocalMessage: openLocalQueuedMessage,
    updateProfilePreferences,
    bindWebAuthn: bindWebAuthnToProfile,
    saveGmailSecret: saveGmailProfileSecret,
    removeGmailSecret: removeGmailProfileSecret,
    saveOAuthSecret: saveOAuthProfileSecret,
    removeOAuthSecret: removeOAuthProfileSecret,
  }
}

export { isAuthenticatedStore, isGuestStore }
