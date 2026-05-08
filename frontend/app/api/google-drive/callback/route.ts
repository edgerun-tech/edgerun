import { NextRequest, NextResponse } from "next/server"
import { runtimeEnv, runtimeEnvAny } from "@/lib/server-runtime-env"

interface TokenResponse {
  access_token: string
  refresh_token?: string
  expires_in: number
  token_type: string
  scope?: string
}

function base64UrlJson(value: unknown): string {
  return Buffer.from(JSON.stringify(value), "utf8").toString("base64url")
}

export async function GET(req: NextRequest) {
  const code = req.nextUrl.searchParams.get("code")
  const state = req.nextUrl.searchParams.get("state")
  const error = req.nextUrl.searchParams.get("error")

  if (error) return NextResponse.redirect(new URL(`/?google_drive_error=${encodeURIComponent(error)}`, req.nextUrl.origin))

  const cookieState = req.cookies.get("google_drive_oauth_state")?.value
  if (!state || state !== cookieState) {
    return NextResponse.json({ error: "Invalid state parameter" }, { status: 400 })
  }
  if (!code) return NextResponse.json({ error: "No authorization code" }, { status: 400 })

  const clientId = await runtimeEnvAny("GOOGLE_CLIENT_ID", "GMAIL_CLIENT_ID")
  const clientSecret = await runtimeEnvAny("GOOGLE_CLIENT_SECRET", "GMAIL_CLIENT_SECRET")
  const redirectUri = await runtimeEnv("GOOGLE_DRIVE_REDIRECT_URI") || `${req.nextUrl.origin}/api/google-drive/callback`
  if (!clientId || !clientSecret) {
    return NextResponse.json({ error: "Google Drive OAuth not configured" }, { status: 500 })
  }

  try {
    const tokenRes = await fetch("https://oauth2.googleapis.com/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        code,
        client_id: clientId,
        client_secret: clientSecret,
        redirect_uri: redirectUri,
        grant_type: "authorization_code",
      }),
    })
    if (!tokenRes.ok) return NextResponse.json({ error: `Token exchange failed: ${await tokenRes.text()}` }, { status: 500 })

    const tokens: TokenResponse = await tokenRes.json()
    const userRes = await fetch("https://www.googleapis.com/oauth2/v2/userinfo?alt=json", {
      headers: { Authorization: `Bearer ${tokens.access_token}` },
    })
    const userInfo = userRes.ok ? await userRes.json() : null
    const res = NextResponse.redirect(new URL("/?google_drive_connected=true", req.nextUrl.origin))
    const secure = process.env.NODE_ENV === "production"

    res.cookies.set("google_drive_access_token", tokens.access_token, {
      httpOnly: true,
      secure,
      sameSite: "lax",
      maxAge: tokens.expires_in,
      path: "/",
    })
    if (tokens.refresh_token) {
      res.cookies.set("google_drive_refresh_token", tokens.refresh_token, {
        httpOnly: true,
        secure,
        sameSite: "lax",
        maxAge: 60 * 60 * 24 * 365,
        path: "/",
      })
    }
    if (userInfo?.email) {
      res.cookies.set("google_drive_email", userInfo.email, {
        httpOnly: false,
        secure,
        sameSite: "lax",
        maxAge: 60 * 60 * 24 * 365,
        path: "/",
      })
    }
    res.cookies.set("google_drive_profile_pending", base64UrlJson({
      email: userInfo?.email || "",
      accessToken: tokens.access_token,
      refreshToken: tokens.refresh_token,
      expiresAtIso: new Date(Date.now() + tokens.expires_in * 1000).toISOString(),
      scopes: tokens.scope?.split(/\s+/).filter(Boolean) ?? [],
    }), {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 600,
      path: "/",
    })
    res.cookies.delete("google_drive_oauth_state")
    return res
  } catch (err) {
    return NextResponse.json({ error: `OAuth callback failed: ${err}` }, { status: 500 })
  }
}
