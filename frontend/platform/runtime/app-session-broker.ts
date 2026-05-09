"use client"

export type AppSessionId = "gmail" | "google-drive" | "github" | "cloudflare"

type BrokerResponse = {
  ok: boolean
  error?: string
  appId?: string
  connected?: boolean
  profileId?: string
  grantId?: string
  expiresAtIso?: string
}

type StoreAppSessionInput = {
  appId: AppSessionId
  accessToken: string
  expiresAtIso?: string
  profileId?: string
  grantId?: string
}

const BROKER_TIMEOUT_MS = 1500

async function appSessionWorker(): Promise<ServiceWorker | null> {
  if (typeof window === "undefined" || !("serviceWorker" in navigator)) return null
  try {
    const registration = await navigator.serviceWorker.register("/session-resume-sw.js", { scope: "/" })
    const ready = await navigator.serviceWorker.ready
    return navigator.serviceWorker.controller ?? ready.active ?? registration.active ?? null
  } catch {
    return null
  }
}

async function postBrokerMessage(message: Record<string, unknown>): Promise<BrokerResponse | null> {
  const worker = await appSessionWorker()
  if (!worker) return null
  return new Promise((resolve) => {
    const channel = new MessageChannel()
    const timeout = window.setTimeout(() => {
      channel.port1.close()
      resolve(null)
    }, BROKER_TIMEOUT_MS)
    channel.port1.onmessage = (event: MessageEvent<BrokerResponse>) => {
      window.clearTimeout(timeout)
      channel.port1.close()
      resolve(event.data)
    }
    worker.postMessage(message, [channel.port2])
  })
}

export async function storeAppSession(input: StoreAppSessionInput): Promise<boolean> {
  if (!input.accessToken.trim()) return false
  const response = await postBrokerMessage({
    type: "STORE_APP_SESSION",
    appId: input.appId,
    accessToken: input.accessToken,
    expiresAtIso: input.expiresAtIso,
    profileId: input.profileId,
    grantId: input.grantId,
  })
  return response?.ok === true
}

export async function deleteAppSession(appId: AppSessionId): Promise<boolean> {
  const response = await postBrokerMessage({ type: "DELETE_APP_SESSION", appId })
  return response?.ok === true
}

export async function clearAppSessions(): Promise<boolean> {
  const response = await postBrokerMessage({ type: "CLEAR_APP_SESSIONS" })
  return response?.ok === true
}

export async function appSessionStatus(appId: AppSessionId): Promise<BrokerResponse | null> {
  return postBrokerMessage({ type: "APP_SESSION_STATUS", appId })
}
