import { NextRequest, NextResponse } from "next/server"
import { APP_SESSION_MAX_AGE_SECONDS } from "../../app-session"

type GoogleDriveSessionRequest = {
  email?: string
  accessToken?: string
  refreshToken?: string
  expiresAtIso?: string
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null) as GoogleDriveSessionRequest | null
  if (!body?.accessToken) {
    return NextResponse.json({ error: "Missing Google Drive access token" }, { status: 400 })
  }

  const secure = process.env.NODE_ENV === "production"
  const res = NextResponse.json({ ok: true, brokerRequired: true })
  if (body.email) {
    res.cookies.set("google_drive_email", body.email, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: APP_SESSION_MAX_AGE_SECONDS,
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
