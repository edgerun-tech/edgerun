import { NextRequest, NextResponse } from "next/server"

interface TokenResponse {
  access_token: string
  refresh_token?: string
  expires_in: number
  token_type: string
}

export async function GET(req: NextRequest) {
  const code = req.nextUrl.searchParams.get("code")
  const state = req.nextUrl.searchParams.get("state")
  const error = req.nextUrl.searchParams.get("error")

  if (error) {
    return NextResponse.redirect(
      new URL(`/dashboard?gmail_error=${error}`, req.nextUrl.origin)
    )
  }

  const cookieState = req.cookies.get("gmail_oauth_state")?.value
  if (!state || state !== cookieState) {
    return NextResponse.json({ error: "Invalid state parameter" }, { status: 400 })
  }

  if (!code) {
    return NextResponse.json({ error: "No authorization code" }, { status: 400 })
  }

  const clientId = process.env.GMAIL_CLIENT_ID
  const clientSecret = process.env.GMAIL_CLIENT_SECRET
  const redirectUri = process.env.GMAIL_REDIRECT_URI || "http://127.0.0.1/api/gmail/callback"

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

    const res = NextResponse.redirect(
      new URL("/dashboard?gmail_connected=true", req.nextUrl.origin)
    )

    // Store tokens in httpOnly cookies (in production, use secure session store)
    res.cookies.set("gmail_access_token", tokens.access_token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: tokens.expires_in,
    })

    if (tokens.refresh_token) {
      res.cookies.set("gmail_refresh_token", tokens.refresh_token, {
        httpOnly: true,
        secure: process.env.NODE_ENV === "production",
        sameSite: "lax",
        maxAge: 60 * 60 * 24 * 365, // 1 year
      })
    }

    if (userInfo?.email) {
      res.cookies.set("gmail_email", userInfo.email, {
        httpOnly: false, // Allow client to read email for display
        secure: process.env.NODE_ENV === "production",
        sameSite: "lax",
        maxAge: 60 * 60 * 24 * 365,
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
