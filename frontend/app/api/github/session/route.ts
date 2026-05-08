import { NextRequest, NextResponse } from "next/server"
import { APP_SESSION_MAX_AGE_SECONDS } from "../../app-session"

type GitHubSessionRequest = {
  email?: string
  accessToken?: string
  expiresAtIso?: string
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null) as GitHubSessionRequest | null
  if (!body?.accessToken) return NextResponse.json({ error: "Missing GitHub access token" }, { status: 400 })

  const secure = process.env.NODE_ENV === "production"
  const res = NextResponse.json({ ok: true, brokerRequired: true })
  if (body.email) {
    res.cookies.set("github_login", body.email, {
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
  res.cookies.delete("github_access_token")
  res.cookies.delete("github_login")
  res.cookies.delete("github_profile_pending")
  return res
}
