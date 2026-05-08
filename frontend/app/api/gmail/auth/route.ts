import { NextRequest, NextResponse } from "next/server"
import { runtimeEnv } from "@/lib/server-runtime-env"

const GMAIL_SCOPES = [
  "https://www.googleapis.com/auth/gmail.readonly",
  "https://www.googleapis.com/auth/gmail.send",
  "https://www.googleapis.com/auth/userinfo.email",
]

export async function GET(req: NextRequest) {
  const clientId = await runtimeEnv("GMAIL_CLIENT_ID")
  const redirectUri = await runtimeEnv("GMAIL_REDIRECT_URI") ||
    `${req.nextUrl.origin}/api/gmail/callback`

  if (!clientId) {
    return NextResponse.json(
      { error: "GMAIL_CLIENT_ID not configured" },
      { status: 500 }
    )
  }

  const state = crypto.randomUUID()
  const params = new URLSearchParams({
    client_id: clientId,
    redirect_uri: redirectUri,
    response_type: "code",
    scope: GMAIL_SCOPES.join(" "),
    access_type: "offline",
    prompt: "consent",
    state,
  })

  const authUrl = `https://accounts.google.com/o/oauth2/v2/auth?${params.toString()}`

  const res = NextResponse.json({ authUrl, state })
  res.cookies.set("gmail_oauth_state", state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    maxAge: 600,
  })

  return res
}
