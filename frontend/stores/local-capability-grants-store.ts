"use client"

import { persistentAtom } from "@nanostores/persistent"

export interface LocalCapabilityGrant {
  appId: string
  capabilityId: string
  grantedAt: number
  expiresAt?: number
  scope: "app"
  reason: string
}

type GrantMap = Record<string, LocalCapabilityGrant[]>

export const localCapabilityGrantsStore = persistentAtom<GrantMap>(
  "edgerun:localCapabilityGrants",
  {},
  {
    encode: JSON.stringify,
    decode: (value) => {
      const parsed = JSON.parse(value)
      return parsed && typeof parsed === "object" ? parsed : {}
    },
  },
)

export function listLocalGrantsForApp(appId: string): LocalCapabilityGrant[] {
  return localCapabilityGrantsStore.get()[appId] ?? []
}

export function hasLocalCapabilityGrant(appId: string, capabilityId: string): boolean {
  const now = Date.now()
  return listLocalGrantsForApp(appId).some((grant) =>
    grant.capabilityId === capabilityId && (!grant.expiresAt || grant.expiresAt > now)
  )
}

export function getMissingCapabilities(appId: string, requiredCapabilityIds: string[]): string[] {
  return requiredCapabilityIds.filter((capabilityId) => !hasLocalCapabilityGrant(appId, capabilityId))
}

export function grantLocalCapabilities(appId: string, capabilityIds: string[], reason: string): void {
  const state = localCapabilityGrantsStore.get()
  const existing = state[appId] ?? []
  const now = Date.now()
  const merged = new Map(existing.map((grant) => [grant.capabilityId, grant]))

  for (const capabilityId of capabilityIds) {
    merged.set(capabilityId, {
      appId,
      capabilityId,
      grantedAt: now,
      scope: "app",
      reason,
    })
  }

  localCapabilityGrantsStore.set({
    ...state,
    [appId]: Array.from(merged.values()),
  })
}

export function revokeLocalCapabilityGrant(appId: string, capabilityId: string): void {
  const state = localCapabilityGrantsStore.get()
  const grants = state[appId] ?? []
  localCapabilityGrantsStore.set({
    ...state,
    [appId]: grants.filter((grant) => grant.capabilityId !== capabilityId),
  })
}

export function revokeAllLocalCapabilityGrants(appId: string): void {
  const state = localCapabilityGrantsStore.get()
  const next = { ...state }
  delete next[appId]
  localCapabilityGrantsStore.set(next)
}
