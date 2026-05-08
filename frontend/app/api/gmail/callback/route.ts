import { NextRequest, NextResponse } from "next/server"
import { runtimeEnv } from "@/lib/server-runtime-env"
import { APP_SESSION_MAX_AGE_SECONDS } from "../../app-session"

interface TokenResponse {
  access_token: string
  refresh_token?: string
  expires_in: number
  token_type: string
  scope?: string
}

function oauthHandoffResponse(storageKey: string, connectedParam: string, secret: unknown) {
  const target = `/?${connectedParam}=true`
  const html = `<!doctype html><meta charset="utf-8"><script>sessionStorage.setItem(${JSON.stringify(storageKey)},${JSON.stringify(JSON.stringify(secret)).replace(/</g, "\\u003c")});location.replace(${JSON.stringify(target)});</script>`
  return new NextResponse(html, { headers: { "Content-Type": "text/html; charset=utf-8" } })
}

export async function GET(req: NextRequest) {
  const code = req.nextUrl.searchParams.get("code")
  const state = req.nextUrl.searchParams.get("state")
  const error = req.nextUrl.searchParams.get("error")

  if (error) {
    return NextResponse.redirect(
      new URL(`/?gmail_error=${error}`, req.nextUrl.origin)
    )
  }

  const cookieState = req.cookies.get("gmail_oauth_state")?.value
  if (!state || state !== cookieState) {
    return NextResponse.json({ error: "Invalid state parameter" }, { status: 400 })
  }

  if (!code) {
    return NextResponse.json({ error: "No authorization code" }, { status: 400 })
  }

  const clientId = await runtimeEnv("GMAIL_CLIENT_ID")
  const clientSecret = await runtimeEnv("GMAIL_CLIENT_SECRET")
  const redirectUri = await runtimeEnv("GMAIL_REDIRECT_URI") || `${req.nextUrl.origin}/api/gmail/callback`

  if (!clientId || !clientSecret) {
    return NextResponse.json(
      { error: "Gmail OAuth not configured" },
      { status: 500 }
    )
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

    if (!tokenRes.ok) {
      const errText = await tokenRes.text()
      return NextResponse.json(
        { error: `Token exchange failed: ${errText}` },
        { status: 500 }
      )
    }

    const tokens: TokenResponse = await tokenRes.json()

    // Get user email
    const userRes = await fetch(
      "https://www.googleapis.com/oauth2/v2/userinfo?alt=json",
      { headers: { Authorization: `Bearer ${tokens.access_token}` } }
    )
    const userInfo = userRes.ok ? await userRes.json() : null

    const res = oauthHandoffResponse("edgerun:oauth-pending:gmail", "gmail_connected", {
      email: userInfo?.email || "",
      accessToken: tokens.access_token,
      refreshToken: tokens.refresh_token,
      expiresAtIso: new Date(Date.now() + tokens.expires_in * 1000).toISOString(),
      scopes: tokens.scope?.split(/\s+/).filter(Boolean) ?? [],
    })

    if (userInfo?.email) {
      res.cookies.set("gmail_email", userInfo.email, {
        httpOnly: false, // Allow client to read email for display
        secure: process.env.NODE_ENV === "production",
        sameSite: "lax",
        maxAge: APP_SESSION_MAX_AGE_SECONDS,
      })
    }

    res.cookies.delete("gmail_oauth_state")

    return res
  } catch (err) {
    return NextResponse.json(
      { error: `OAuth callback failed: ${err}` },
      { status: 500 }
    )
  }
}
