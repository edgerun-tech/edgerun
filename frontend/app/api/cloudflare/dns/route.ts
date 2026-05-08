import { NextRequest } from "next/server"
import { cloudflareFetch } from "../cloudflare"

function zonePath(req: NextRequest): string | null {
  const zoneId = req.nextUrl.searchParams.get("zoneId") || req.cookies.get("cloudflare_zone_id")?.value
  if (!zoneId || !/^[a-f0-9]{32}$/i.test(zoneId)) return null
  return `/zones/${zoneId}/dns_records`
}

export async function GET(req: NextRequest) {
  const path = zonePath(req)
  if (!path) return Response.json({ error: "Missing or invalid zoneId" }, { status: 400 })
  const params = new URLSearchParams({
    per_page: "100",
    page: req.nextUrl.searchParams.get("page") || "1",
  })
  const type = req.nextUrl.searchParams.get("type")
  const name = req.nextUrl.searchParams.get("name")
  if (type) params.set("type", type)
  if (name) params.set("name", name)
  return cloudflareFetch(req, `${path}?${params.toString()}`)
}

export async function POST(req: NextRequest) {
  const path = zonePath(req)
  if (!path) return Response.json({ error: "Missing or invalid zoneId" }, { status: 400 })
  const body = await req.json().catch(() => null)
  return cloudflareFetch(req, path, { method: "POST", body: JSON.stringify(body ?? {}) })
}

export async function PATCH(req: NextRequest) {
  const path = zonePath(req)
  const recordId = req.nextUrl.searchParams.get("recordId")
  if (!path || !recordId) return Response.json({ error: "Missing zoneId or recordId" }, { status: 400 })
  const body = await req.json().catch(() => null)
  return cloudflareFetch(req, `${path}/${recordId}`, { method: "PATCH", body: JSON.stringify(body ?? {}) })
}

export async function DELETE(req: NextRequest) {
  const path = zonePath(req)
  const recordId = req.nextUrl.searchParams.get("recordId")
  if (!path || !recordId) return Response.json({ error: "Missing zoneId or recordId" }, { status: 400 })
  return cloudflareFetch(req, `${path}/${recordId}`, { method: "DELETE" })
}
