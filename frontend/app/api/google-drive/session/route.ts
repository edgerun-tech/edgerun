import { NextRequest, NextResponse } from "next/server"

type GoogleDriveSessionRequest = {
  email?: string
  accessToken?: string
  refreshToken?: string
  expiresAtIso?: string
}

function maxAgeFromIso(iso?: string): number {
  if (!iso) return 3600
  const milliseconds = new Date(iso).getTime() - Date.now()
  if (!Number.isFinite(milliseconds) || milliseconds <= 0) return 60
  return Math.max(60, Math.floor(milliseconds / 1000))
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null) as GoogleDriveSessionRequest | null
  if (!body?.accessToken) {
    return NextResponse.json({ error: "Missing Google Drive access token" }, { status: 400 })
  }

  const secure = process.env.NODE_ENV === "production"
  const res = NextResponse.json({ ok: true })
  res.cookies.set("google_drive_access_token", body.accessToken, {
    httpOnly: true,
    secure,
    sameSite: "lax",
    maxAge: maxAgeFromIso(body.expiresAtIso),
    path: "/",
  })
  if (body.refreshToken) {
    res.cookies.set("google_drive_refresh_token", body.refreshToken, {
      httpOnly: true,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
  }
  if (body.email) {
    res.cookies.set("google_drive_email", body.email, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
  }
  return res
}

export async function DELETE() {
  const res = NextResponse.json({ ok: true })
  res.cookies.delete("google_drive_access_token")
  res.cookies.delete("google_drive_refresh_token")
  res.cookies.delete("google_drive_email")
  res.cookies.delete("google_drive_profile_pending")
  return res
}
