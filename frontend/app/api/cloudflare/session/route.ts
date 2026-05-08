import { NextRequest, NextResponse } from "next/server"
import { APP_SESSION_MAX_AGE_SECONDS } from "../../app-session"
import { CLOUDFLARE_API_BASE, cloudflareError, cloudflareHeaders } from "../cloudflare"

type CloudflareSessionRequest = {
  email?: string
  accessToken?: string
  expiresAtIso?: string
  accountId?: string
  zoneId?: string
  zoneName?: string
}

function validZoneId(zoneId?: string): zoneId is string {
  return Boolean(zoneId && /^[a-f0-9]{32}$/i.test(zoneId))
}

function validCloudflareId(id?: string): id is string {
  return Boolean(id && /^[a-f0-9]{32}$/i.test(id))
}

export async function POST(req: NextRequest) {
  const brokerMode = req.headers.get("x-edgerun-app-session-mode") === "broker"
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
  if (brokerMode) return res
  res.cookies.set("cloudflare_label", label, {
    httpOnly: false,
    secure,
    sameSite: "lax",
    maxAge: APP_SESSION_MAX_AGE_SECONDS,
    path: "/",
  })
  if (tokenId) {
    res.cookies.set("cloudflare_token_id", tokenId, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: APP_SESSION_MAX_AGE_SECONDS,
      path: "/",
    })
  }
  if (validCloudflareId(body.accountId)) {
    res.cookies.set("cloudflare_account_id", body.accountId, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: APP_SESSION_MAX_AGE_SECONDS,
      path: "/",
    })
  }
  if (validZoneId(body.zoneId)) {
    res.cookies.set("cloudflare_zone_id", body.zoneId, {
      httpOnly: false,
      secure,
      sameSite: "lax",
      maxAge: APP_SESSION_MAX_AGE_SECONDS,
      path: "/",
    })
    if (verifiedZoneName) {
      res.cookies.set("cloudflare_zone_name", verifiedZoneName, {
        httpOnly: false,
        secure,
        sameSite: "lax",
        maxAge: APP_SESSION_MAX_AGE_SECONDS,
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
