import { NextRequest, NextResponse } from "next/server"
import { CLOUDFLARE_API_BASE, cloudflareError, cloudflareHeaders } from "../cloudflare"

type CloudflareSessionRequest = {
  email?: string
  accessToken?: string
  expiresAtIso?: string
}

function maxAgeFromIso(iso?: string): number {
  if (!iso) return 60 * 60 * 24 * 365
  const milliseconds = new Date(iso).getTime() - Date.now()
  if (!Number.isFinite(milliseconds) || milliseconds <= 0) return 60
  return Math.max(60, Math.floor(milliseconds / 1000))
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null) as CloudflareSessionRequest | null
  if (!body?.accessToken) return NextResponse.json({ error: "Missing Cloudflare API token" }, { status: 400 })

  const verify = await fetch(`${CLOUDFLARE_API_BASE}/user/tokens/verify`, {
    headers: cloudflareHeaders(body.accessToken),
  })
  const verifyData = await verify.json().catch(() => null)
  if (!verify.ok || verifyData?.success === false) {
    return NextResponse.json({ error: cloudflareError(verifyData, "Cloudflare token verification failed") }, { status: verify.status || 400 })
  }

  const secure = process.env.NODE_ENV === "production"
  const label = body.email || verifyData?.result?.id || "Cloudflare API token"
  const res = NextResponse.json({ ok: true, label, tokenStatus: verifyData?.result?.status ?? "active" })
  res.cookies.set("cloudflare_api_token", body.accessToken, {
    httpOnly: true,
    secure,
    sameSite: "lax",
    maxAge: maxAgeFromIso(body.expiresAtIso),
    path: "/",
  })
  res.cookies.set("cloudflare_label", label, {
    httpOnly: false,
    secure,
    sameSite: "lax",
    maxAge: 60 * 60 * 24 * 365,
    path: "/",
  })
  return res
}

export async function DELETE() {
  const res = NextResponse.json({ ok: true })
  res.cookies.delete("cloudflare_api_token")
  res.cookies.delete("cloudflare_label")
  return res
}
