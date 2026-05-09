export const APP_SESSION_MAX_AGE_SECONDS = 5 * 60

export function appSessionMaxAge(expiresAtIso?: string): number {
  if (!expiresAtIso) return APP_SESSION_MAX_AGE_SECONDS
  const seconds = Math.floor((new Date(expiresAtIso).getTime() - Date.now()) / 1000)
  if (!Number.isFinite(seconds) || seconds <= 0) return 60
  return Math.max(60, Math.min(APP_SESSION_MAX_AGE_SECONDS, seconds))
}

export function oauthProviderMaxAge(expiresIn?: number): number {
  if (!Number.isFinite(expiresIn) || !expiresIn || expiresIn <= 0) return APP_SESSION_MAX_AGE_SECONDS
  return Math.max(60, Math.min(APP_SESSION_MAX_AGE_SECONDS, Math.floor(expiresIn)))
}
