/* global self, btoa, atob */

const tickets = new Map()
const appSessions = new Map()
const APP_SESSION_MAX_TTL_MS = 5 * 60 * 1000

const appSessionRoutes = [
  { prefix: "/api/gmail/", appId: "gmail" },
  { prefix: "/api/google-drive/", appId: "google-drive" },
  { prefix: "/api/google-photos/", appId: "google-drive" },
  { prefix: "/api/google-contacts/", appId: "google-drive" },
  { prefix: "/api/github/", appId: "github" },
  { prefix: "/api/cloudflare/", appId: "cloudflare" },
]

const appSessionControlPaths = new Set([
  "/api/gmail/auth",
  "/api/gmail/callback",
  "/api/gmail/session",
  "/api/google-drive/auth",
  "/api/google-drive/callback",
  "/api/google-drive/session",
  "/api/github/auth",
  "/api/github/callback",
  "/api/github/session",
  "/api/cloudflare/session",
])

function bytesToBase64(bytes) {
  let binary = ""
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.slice(offset, offset + 0x8000))
  }
  return btoa(binary)
}

function base64ToBytes(base64) {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index)
  return bytes
}

async function importAesKey(raw) {
  return crypto.subtle.importKey("raw", raw, { name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"])
}

function deleteExpiredTickets() {
  const now = Date.now()
  for (const [handle, ticket] of tickets) {
    if (ticket.expiresAtMs <= now) tickets.delete(handle)
  }
}

function deleteExpiredAppSessions() {
  const now = Date.now()
  for (const [appId, session] of appSessions) {
    if (session.expiresAtMs <= now) appSessions.delete(appId)
  }
}

function appIdForRequest(url) {
  if (url.origin !== self.location.origin) return null
  if (appSessionControlPaths.has(url.pathname)) return null
  const route = appSessionRoutes.find((entry) => url.pathname.startsWith(entry.prefix))
  return route?.appId || null
}

function clampExpiry(expiresAtIso) {
  const requested = Date.parse(String(expiresAtIso || ""))
  const now = Date.now()
  if (Number.isFinite(requested) && requested > now) {
    return Math.min(requested, now + APP_SESSION_MAX_TTL_MS)
  }
  return now + APP_SESSION_MAX_TTL_MS
}

self.addEventListener("install", (event) => {
  event.waitUntil(self.skipWaiting())
})

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim())
})

self.addEventListener("message", (event) => {
  const port = event.ports[0]
  const message = event.data || {}
  if (!port) return

  event.waitUntil((async () => {
    try {
      deleteExpiredTickets()
      if (message.type === "STORE_SESSION_RESUME") {
        if (message.previousHandle) tickets.delete(message.previousHandle)
        const handle = crypto.randomUUID()
        const keyBytes = crypto.getRandomValues(new Uint8Array(32))
        const iv = crypto.getRandomValues(new Uint8Array(12))
        const key = await importAesKey(keyBytes)
        const expiresAtMs = Date.now() + Number(message.ttlMs || 30_000)
        const ciphertext = new Uint8Array(await crypto.subtle.encrypt(
          { name: "AES-GCM", iv },
          key,
          new TextEncoder().encode(String(message.profileJson || "")),
        ))
        tickets.set(handle, {
          profileId: String(message.profileId || ""),
          expiresAtMs,
          key,
          ivBase64: bytesToBase64(iv),
          ciphertextBase64: bytesToBase64(ciphertext),
        })
        port.postMessage({ ok: true, handle, profileId: message.profileId, expiresAtIso: new Date(expiresAtMs).toISOString() })
        return
      }

      if (message.type === "CONSUME_SESSION_RESUME") {
        const handle = String(message.handle || "")
        const ticket = tickets.get(handle)
        tickets.delete(handle)
        if (!ticket || ticket.expiresAtMs <= Date.now()) {
          port.postMessage({ ok: false })
          return
        }
        const plaintext = await crypto.subtle.decrypt(
          { name: "AES-GCM", iv: base64ToBytes(ticket.ivBase64) },
          ticket.key,
          base64ToBytes(ticket.ciphertextBase64),
        )
        port.postMessage({
          ok: true,
          profileId: ticket.profileId,
          profileJson: new TextDecoder().decode(plaintext),
        })
        return
      }

      if (message.type === "CLEAR_SESSION_RESUME") {
        if (message.handle) tickets.delete(String(message.handle))
        port.postMessage({ ok: true })
        return
      }

      if (message.type === "STORE_APP_SESSION") {
        if (!message.appId || !message.accessToken) {
          port.postMessage({ ok: false, error: "missing app session identity or token" })
          return
        }
        const appId = String(message.appId)
        const expiresAtMs = clampExpiry(message.expiresAtIso)
        appSessions.set(appId, {
          appId,
          profileId: String(message.profileId || ""),
          grantId: String(message.grantId || ""),
          accessToken: String(message.accessToken),
          expiresAtMs,
        })
        port.postMessage({ ok: true, appId, expiresAtIso: new Date(expiresAtMs).toISOString() })
        return
      }

      if (message.type === "DELETE_APP_SESSION") {
        if (message.appId) appSessions.delete(String(message.appId))
        port.postMessage({ ok: true })
        return
      }

      if (message.type === "CLEAR_APP_SESSIONS") {
        appSessions.clear()
        port.postMessage({ ok: true })
        return
      }

      if (message.type === "APP_SESSION_STATUS") {
        deleteExpiredAppSessions()
        const session = message.appId ? appSessions.get(String(message.appId)) : null
        port.postMessage({
          ok: true,
          appId: message.appId ? String(message.appId) : "",
          connected: Boolean(session),
          profileId: session?.profileId || "",
          grantId: session?.grantId || "",
          expiresAtIso: session ? new Date(session.expiresAtMs).toISOString() : "",
        })
        return
      }

      port.postMessage({ ok: false, error: "unknown message type" })
    } catch (error) {
      port.postMessage({ ok: false, error: error instanceof Error ? error.message : String(error) })
    }
  })())
})

self.addEventListener("fetch", (event) => {
  if (event.request.method === "OPTIONS") return
  const url = new URL(event.request.url)
  const appId = appIdForRequest(url)
  if (!appId) return

  event.respondWith((async () => {
    deleteExpiredAppSessions()
    const session = appSessions.get(appId)
    if (!session) return fetch(event.request)

    const headers = new Headers(event.request.headers)
    headers.set("Authorization", `Bearer ${session.accessToken}`)
    headers.set("X-Edgerun-App-Session", "broker-v1")
    headers.set("X-Edgerun-App-Id", session.appId)
    if (session.profileId) headers.set("X-Edgerun-Profile-Id", session.profileId)
    if (session.grantId) headers.set("X-Edgerun-Grant-Id", session.grantId)

    const mediated = new Request(event.request, { headers })
    return fetch(mediated)
  })())
})
