import { NextRequest, NextResponse } from "next/server"
import { runtimeEnv, runtimeEnvAny } from "@/lib/server-runtime-env"

const DRIVE_SCOPES = [
  "https://www.googleapis.com/auth/drive.metadata.readonly",
  "https://www.googleapis.com/auth/drive.readonly",
  "https://www.googleapis.com/auth/drive.file",
  "https://www.googleapis.com/auth/contacts.readonly",
  "https://www.googleapis.com/auth/photospicker.mediaitems.readonly",
  "https://www.googleapis.com/auth/userinfo.email",
]

export async function GET(req: NextRequest) {
  const clientId = await runtimeEnvAny("GOOGLE_CLIENT_ID", "GMAIL_CLIENT_ID")
  const redirectUri = await runtimeEnv("GOOGLE_DRIVE_REDIRECT_URI") ||
    `${req.nextUrl.origin}/api/google-drive/callback`

  if (!clientId) {
    return NextResponse.json({ error: "Google OAuth client ID not configured" }, { status: 500 })
  }

  const state = crypto.randomUUID()
  const params = new URLSearchParams({
    client_id: clientId,
    redirect_uri: redirectUri,
    response_type: "code",
    scope: DRIVE_SCOPES.join(" "),
    access_type: "offline",
    prompt: "consent",
    state,
  })

  const res = NextResponse.json({ authUrl: `https://accounts.google.com/o/oauth2/v2/auth?${params.toString()}`, state })
  res.cookies.set("google_drive_oauth_state", state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    maxAge: 600,
    path: "/",
  })
  return res
}
