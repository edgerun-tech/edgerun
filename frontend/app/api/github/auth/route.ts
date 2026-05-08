import { NextRequest, NextResponse } from "next/server"
import { runtimeEnv, runtimeEnvAny } from "@/lib/server-runtime-env"

const GITHUB_SCOPES = ["read:user", "user:email", "repo"]

export async function GET(req: NextRequest) {
  const clientId = await runtimeEnvAny("GITHUB_CLIENT_ID", "GITHUB_OAUTH_ID")
  const redirectUri = await runtimeEnv("GITHUB_REDIRECT_URI") || `${req.nextUrl.origin}/api/github/callback`
  if (!clientId) {
    return NextResponse.json({ error: "GitHub OAuth client ID not configured" }, { status: 500 })
  }

  const state = crypto.randomUUID()
  const params = new URLSearchParams({
    client_id: clientId,
    redirect_uri: redirectUri,
    scope: GITHUB_SCOPES.join(" "),
    state,
  })
  const res = NextResponse.json({ authUrl: `https://github.com/login/oauth/authorize?${params.toString()}`, state })
  res.cookies.set("github_oauth_state", state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    maxAge: 600,
    path: "/",
  })
  return res
}
