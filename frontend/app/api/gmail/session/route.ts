import { NextRequest, NextResponse } from "next/server"
import { APP_SESSION_MAX_AGE_SECONDS } from "../../app-session"

type GmailSessionRequest = {
  email?: string
  accessToken?: string
  refreshToken?: string
  expiresAtIso?: string
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null) as GmailSessionRequest | null
  if (!body?.accessToken) {
    return NextResponse.json({ error: "Missing Gmail access token" }, { status: 400 })
  }

  const res = NextResponse.json({ ok: true, brokerRequired: true })
  if (body.email) {
    res.cookies.set("gmail_email", body.email, {
      httpOnly: false,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: APP_SESSION_MAX_AGE_SECONDS,
      path: "/",
    })
  }
  return res
}

export async function DELETE() {
  const res = NextResponse.json({ ok: true })
  res.cookies.delete("gmail_access_token")
  res.cookies.delete("gmail_refresh_token")
  res.cookies.delete("gmail_email")
  res.cookies.delete("gmail_profile_pending")
  return res
}
