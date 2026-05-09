import { NextResponse } from "next/server"

const APP_SESSION_COOKIES = [
  "gmail_access_token",
  "gmail_refresh_token",
  "gmail_email",
  "gmail_profile_pending",
  "gmail_oauth_state",
  "google_drive_access_token",
  "google_drive_refresh_token",
  "google_drive_email",
  "google_drive_profile_pending",
  "google_drive_oauth_state",
  "github_access_token",
  "github_login",
  "github_profile_pending",
  "github_oauth_state",
  "cloudflare_api_token",
  "cloudflare_label",
  "cloudflare_account_id",
  "cloudflare_token_id",
  "cloudflare_zone_id",
  "cloudflare_zone_name",
]

function clearAppSessionCookies() {
  const res = NextResponse.json({ ok: true })
  for (const name of APP_SESSION_COOKIES) res.cookies.delete(name)
  return res
}

export function POST() {
  return clearAppSessionCookies()
}

export function DELETE() {
  return clearAppSessionCookies()
}
