import { NextRequest, NextResponse } from "next/server"
import { CLOUDFLARE_API_BASE, cloudflareError, cloudflareHeaders } from "../cloudflare"

type CloudflareSessionRequest = {
  email?: string
  accessToken?: string
  expiresAtIso?: string
  accountId?: string
  zoneId?: string
  zoneName?: string
}

function maxAgeFromIso(iso?: string): number {
  if (!iso) return 60 * 60 * 24 * 365
  const milliseconds = new Date(iso).getTime() - Date.now()
  if (!Number.isFinite(milliseconds) || milliseconds <= 0) return 60
  return Math.max(60, Math.floor(milliseconds / 1000))
}

function validZoneId(zoneId?: string): zoneId is string {
  return Boolean(zoneId && /^[a-f0-9]{32}$/i.test(zoneId))
}

function validCloudflareId(id?: string): id is string {
  return Boolean(id && /^[a-f0-9]{32}$/i.test(id))
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null) as CloudflareSessionRequest | null
  if (!body?.accessToken) return NextResponse.json({ error: "Missing Cloudflare API token" }, { status: 400 })
  if (body.accountId && !validCloudflareId(body.accountId)) return NextResponse.json({ error: "Invalid Cloudflare account ID" }, { status: 400 })
  if (body.zoneId && !validZoneId(body.zoneId)) return NextResponse.json({ error: "Invalid Cloudflare zone ID" }, { status: 400 })

  const verifyPath = body.accountId ? `/accounts/${body.accountId}/tokens/verify` : "/user/tokens/verify"
  const verify = await fetch(`${CLOUDFLARE_API_BASE}${verifyPath}`, {
    headers: cloudflareHeaders(body.accessToken),
  })
  const verifyData = await verify.json().catch(() => null)
  if (!verify.ok || verifyData?.success === false) {
    return NextResponse.json({ error: cloudflareError(verifyData, "Cloudflare token verification failed") }, { status: verify.status || 400 })
  }

  let verifiedZoneName = body.zoneName
  if (validZoneId(body.zoneId)) {
    const zone = await fetch(`${CLOUDFLARE_API_BASE}/zones/${body.zoneId}`, {
      headers: cloudflareHeaders(body.accessToken),
    })
    const zoneData = await zone.json().catch(() => null)
    if (!zone.ok || zoneData?.success === false || zoneData?.result?.id !== body.zoneId) {
      return NextResponse.json({ error: cloudflareError(zoneData, "Cloudflare zone verification failed"), needZone: true }, { status: zone.status || 400 })
    }
    verifiedZoneName = typeof zoneData?.result?.name === "string" ? zoneData.result.name : verifiedZoneName
  }

  const secure = process.env.NODE_ENV === "production"
  const tokenId = typeof verifyData?.result?.id === "string" ? verifyData.result.id : null
  const label = body.email || "Cloudflare API token"
  const res = NextResponse.json({ ok: true, label, accountId: body.accountId ?? null, tokenId, zoneId: body.zoneId ?? null, zoneName: verifiedZoneName ?? null, tokenStatus: verifyData?.result?.status ?? "active" })
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
  if (tokenId) {
    res.cookies.set("cloudflare_token_id", tokenId, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
  }
  if (validCloudflareId(body.accountId)) {
    res.cookies.set("cloudflare_account_id", body.accountId, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
  }
  if (validZoneId(body.zoneId)) {
    res.cookies.set("cloudflare_zone_id", body.zoneId, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 365,
      path: "/",
    })
    if (verifiedZoneName) {
      res.cookies.set("cloudflare_zone_name", verifiedZoneName, {
        httpOnly: false,
        secure,
        sameSite: "lax",
        maxAge: 60 * 60 * 24 * 365,
        path: "/",
      })
    }
  }
  return res
}

export async function DELETE() {
  const res = NextResponse.json({ ok: true })
  res.cookies.delete("cloudflare_api_token")
  res.cookies.delete("cloudflare_label")
  res.cookies.delete("cloudflare_account_id")
  res.cookies.delete("cloudflare_token_id")
  res.cookies.delete("cloudflare_zone_id")
  res.cookies.delete("cloudflare_zone_name")
  return res
}
