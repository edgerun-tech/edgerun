const SESSION_RESUME_KEY = "edgerun:session_resume"
const SESSION_RESUME_TTL_MS = 300_000
const SESSION_RESUME_REFRESH_MS = 60_000

type SessionResumeTicket = {
  version: 1
  profileId: string
  expiresAtIso: string
  handle: string
}

type SessionResumeWorkerResponse = {
  ok: boolean
  handle?: string
  profileId?: string
  expiresAtIso?: string
  profileJson?: string
  error?: string
}

let sessionResumeTimer: ReturnType<typeof setInterval> | null = null

async function sessionResumeWorker(): Promise<ServiceWorker | null> {
  if (typeof window === "undefined" || !("serviceWorker" in navigator)) return null
  try {
    const registration = await navigator.serviceWorker.register("/session-resume-sw.js", { scope: "/" })
    const ready = await navigator.serviceWorker.ready
    return ready.active ?? registration.active ?? null
  } catch {
    return null
  }
}

async function postSessionResumeWorker(message: Record<string, unknown>): Promise<SessionResumeWorkerResponse | null> {
  const worker = await sessionResumeWorker()
  if (!worker) return null
  return new Promise((resolve) => {
    const channel = new MessageChannel()
    const timeout = window.setTimeout(() => {
      channel.port1.close()
      resolve(null)
    }, 1500)
    channel.port1.onmessage = (event: MessageEvent<SessionResumeWorkerResponse>) => {
      window.clearTimeout(timeout)
      channel.port1.close()
      resolve(event.data)
    }
    worker.postMessage(message, [channel.port2])
  })
}

function readTicket(): SessionResumeTicket | null {
  if (typeof window === "undefined") return null
  const raw = sessionStorage.getItem(SESSION_RESUME_KEY)
  if (!raw) return null
  try {
    return JSON.parse(raw) as SessionResumeTicket
  } catch {
    return null
  }
}

function writeTicket(ticket: SessionResumeTicket) {
  sessionStorage.setItem(SESSION_RESUME_KEY, JSON.stringify(ticket))
}

function clearTicket() {
  sessionStorage.removeItem(SESSION_RESUME_KEY)
}

export function clearSessionResumeTicket() {
  if (typeof window === "undefined") return
  if (sessionResumeTimer) {
    clearInterval(sessionResumeTimer)
    sessionResumeTimer = null
  }
  const ticket = readTicket()
  if (ticket?.handle) {
    postSessionResumeWorker({ type: "CLEAR_SESSION_RESUME", handle: ticket.handle }).catch(() => undefined)
  }
  clearTicket()
}

export async function writeSessionResumeTicket(profileId: string, profileJson: string, handle: string) {
  if (typeof window === "undefined") return
  const previousTicket = readTicket()
  const response = await postSessionResumeWorker({
    type: "STORE_SESSION_RESUME",
    profileId,
    profileJson,
    ttlMs: SESSION_RESUME_TTL_MS,
    previousHandle: previousTicket?.handle,
  })
  if (!response?.ok || !response.handle || !response.expiresAtIso) {
    clearTicket()
    return
  }
  writeTicket({
    version: 1,
    profileId,
    expiresAtIso: response.expiresAtIso,
    handle: response.handle,
  })
}

export function startSessionResumeHeartbeat(
  profileId: string,
  profileJson: () => string,
  handle: string,
  isActive: () => boolean,
) {
  if (typeof window === "undefined") return
  if (sessionResumeTimer) clearInterval(sessionResumeTimer)
  writeSessionResumeTicket(profileId, profileJson(), handle)
  sessionResumeTimer = setInterval(() => {
    if (!isActive()) {
      clearSessionResumeTicket()
      return
    }
    writeSessionResumeTicket(profileId, profileJson(), handle)
  }, SESSION_RESUME_REFRESH_MS)
}

export function stopSessionResumeHeartbeat() {
  clearSessionResumeTicket()
}

export async function consumeSessionResumeTicket(
  expectedProfileId: string,
  getSealedProfileId: () => string | null,
  importKey: () => Promise<void>,
  normalize: (profile: unknown) => unknown,
): Promise<unknown | null> {
  if (typeof window === "undefined") return null
  const raw = sessionStorage.getItem(SESSION_RESUME_KEY)
  if (!raw) return null
  sessionStorage.removeItem(SESSION_RESUME_KEY)
  try {
    const ticket = JSON.parse(raw) as SessionResumeTicket
    if (ticket.version !== 1 || !ticket.profileId || !ticket.handle || Date.parse(ticket.expiresAtIso) <= Date.now()) return null
    const response = await postSessionResumeWorker({ type: "CONSUME_SESSION_RESUME", handle: ticket.handle })
    if (!response?.ok || response.profileId !== ticket.profileId || !response.profileJson) return null
    const profile = JSON.parse(response.profileJson)
    const sealedProfileId = getSealedProfileId()
    if (!sealedProfileId || profile.ownerEncryption?.identityIdHex !== ticket.profileId) return null
    if (profile.ownerEncryption?.identityIdHex !== expectedProfileId) return null
    await importKey()
    return normalize(profile)
  } catch {
    return null
  }
}
