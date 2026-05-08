const tickets = new Map()

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

      port.postMessage({ ok: false, error: "unknown message type" })
    } catch (error) {
      port.postMessage({ ok: false, error: error instanceof Error ? error.message : String(error) })
    }
  })())
})
