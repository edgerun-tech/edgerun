export function readCookie(name: string): string | null {
  const prefix = `${name}=`
  return document.cookie.split("; ").find((cookie) => cookie.startsWith(prefix))?.slice(prefix.length) ?? null
}

export function deleteCookie(name: string, sessionKey?: string) {
  document.cookie = `${name}=; Max-Age=0; path=/`
  if (sessionKey) sessionStorage.removeItem(sessionKey)
}

export function writeCookie(name: string, value: string) {
  document.cookie = `${name}=${encodeURIComponent(value)}; Max-Age=31536000; path=/; SameSite=Lax`
}

export type PendingOAuthSecret = {
  email: string
  accessToken: string
  refreshToken?: string
  expiresAtIso: string
  scopes: string[]
}

export function readPendingOAuthSecret(storageKey: string, cookieName: string): PendingOAuthSecret | null {
  const stored = sessionStorage.getItem(storageKey)
  if (stored) {
    try {
      const parsed = JSON.parse(stored) as PendingOAuthSecret
      return parsed.accessToken && parsed.expiresAtIso ? { ...parsed, scopes: parsed.scopes ?? [] } : null
    } catch {
      sessionStorage.removeItem(storageKey)
    }
  }
  const raw = readCookie(cookieName)
  if (!raw) return null
  try {
    const parsed = JSON.parse(atob(raw.replace(/-/g, "+").replace(/_/g, "/"))) as PendingOAuthSecret
    return parsed.accessToken && parsed.expiresAtIso ? { ...parsed, scopes: parsed.scopes ?? [] } : null
  } catch {
    deleteCookie(cookieName, storageKey)
    return null
  }
}

export function GoogleMark({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
      <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
      <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
      <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
    </svg>
  )
}
