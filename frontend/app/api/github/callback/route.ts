import { NextRequest, NextResponse } from "next/server"
import { runtimeEnv, runtimeEnvAny } from "@/lib/server-runtime-env"
import { githubHeaders, GITHUB_USER_AGENT } from "../github-headers"

type GitHubTokenResponse = {
  access_token?: string
  scope?: string
  token_type?: string
  error?: string
  error_description?: string
}

function base64UrlJson(value: unknown): string {
  return Buffer.from(JSON.stringify(value), "utf8").toString("base64url")
}

export async function GET(req: NextRequest) {
  const code = req.nextUrl.searchParams.get("code")
  const state = req.nextUrl.searchParams.get("state")
  const error = req.nextUrl.searchParams.get("error")
  if (error) return NextResponse.redirect(new URL(`/?github_error=${encodeURIComponent(error)}`, req.nextUrl.origin))

  const cookieState = req.cookies.get("github_oauth_state")?.value
  if (!state || state !== cookieState) return NextResponse.json({ error: "Invalid state parameter" }, { status: 400 })
  if (!code) return NextResponse.json({ error: "No authorization code" }, { status: 400 })

  const clientId = await runtimeEnvAny("GITHUB_CLIENT_ID", "GITHUB_OAUTH_ID")
  const clientSecret = await runtimeEnvAny("GITHUB_CLIENT_SECRET", "GITHUB_OAUTH_SECRET")
  const redirectUri = await runtimeEnv("GITHUB_REDIRECT_URI") || `${req.nextUrl.origin}/api/github/callback`
  if (!clientId || !clientSecret) return NextResponse.json({ error: "GitHub OAuth not configured" }, { status: 500 })

  try {
    const tokenRes = await fetch("https://github.com/login/oauth/access_token", {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "User-Agent": GITHUB_USER_AGENT,
      },
      body: JSON.stringify({
        code,
        client_id: clientId,
        client_secret: clientSecret,
        redirect_uri: redirectUri,
      }),
    })
    const tokens: GitHubTokenResponse = await tokenRes.json()
    if (!tokenRes.ok || !tokens.access_token) {
      return NextResponse.json({ error: tokens.error_description || tokens.error || "GitHub token exchange failed" }, { status: 500 })
    }

    const userRes = await fetch("https://api.github.com/user", {
      headers: githubHeaders(tokens.access_token),
    })
    const user = userRes.ok ? await userRes.json() : null
    const emailsRes = await fetch("https://api.github.com/user/emails", {
      headers: githubHeaders(tokens.access_token),
    })
    const emails = emailsRes.ok ? await emailsRes.json() : []
    const primaryEmail = Array.isArray(emails)
      ? emails.find((item) => item?.primary && typeof item?.email === "string")?.email
        ?? emails.find((item) => typeof item?.email === "string")?.email
      : undefined
    const login = typeof user?.login === "string" && user.login ? user.login : primaryEmail || "GitHub"
    const email = typeof user?.email === "string" && user.email ? user.email : primaryEmail || login
    const secure = process.env.NODE_ENV === "production"
    const res = NextResponse.redirect(new URL("/?github_connected=true", req.nextUrl.origin))

    res.cookies.set("github_access_token", tokens.access_token, {
      httpOnly: true,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
    res.cookies.set("github_login", login, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
    res.cookies.set("github_profile_pending", base64UrlJson({
      email,
      accessToken: tokens.access_token,
      expiresAtIso: new Date(Date.now() + 60 * 60 * 24 * 365 * 1000).toISOString(),
      scopes: tokens.scope?.split(",").filter(Boolean) ?? [],
    }), {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 600,
      path: "/",
    })
    res.cookies.delete("github_oauth_state")
    return res
  } catch (err) {
    return NextResponse.json({ error: `GitHub OAuth callback failed: ${err}` }, { status: 500 })
  }
}
