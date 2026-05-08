import { NextRequest, NextResponse } from "next/server"
import { mediatedProviderToken } from "../../provider-auth"

export async function POST(req: NextRequest) {
  const token = mediatedProviderToken(req, "google_drive_access_token")
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  try {
    const photosRes = await fetch("https://photospicker.googleapis.com/v1/sessions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: "{}",
    })
    if (!photosRes.ok) {
      if (photosRes.status === 401) return NextResponse.json({ error: "Token expired", needReauth: true }, { status: 401 })
      return NextResponse.json({ error: `Google Photos Picker session failed: ${await photosRes.text()}` }, { status: photosRes.status })
    }
    return NextResponse.json(await photosRes.json())
  } catch (err) {
    return NextResponse.json({ error: `Failed to create Google Photos Picker session: ${err}` }, { status: 500 })
  }
}

export async function GET(req: NextRequest) {
  const token = mediatedProviderToken(req, "google_drive_access_token")
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  const sessionId = req.nextUrl.searchParams.get("sessionId")
  if (!sessionId) return NextResponse.json({ error: "Missing sessionId" }, { status: 400 })
  const listItems = req.nextUrl.searchParams.get("mediaItems") === "true"
  const pageToken = req.nextUrl.searchParams.get("pageToken")
  const params = new URLSearchParams({ sessionId })
  if (pageToken) params.set("pageToken", pageToken)

  try {
    const url = listItems
      ? `https://photospicker.googleapis.com/v1/mediaItems?${params.toString()}`
      : `https://photospicker.googleapis.com/v1/sessions/${encodeURIComponent(sessionId)}`
    const photosRes = await fetch(url, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (!photosRes.ok) {
      if (photosRes.status === 401) return NextResponse.json({ error: "Token expired", needReauth: true }, { status: 401 })
      return NextResponse.json({ error: `Google Photos Picker API error: ${await photosRes.text()}` }, { status: photosRes.status })
    }
    return NextResponse.json(await photosRes.json())
  } catch (err) {
    return NextResponse.json({ error: `Failed to fetch Google Photos Picker data: ${err}` }, { status: 500 })
  }
}

export async function DELETE(req: NextRequest) {
  const token = mediatedProviderToken(req, "google_drive_access_token")
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  const sessionId = req.nextUrl.searchParams.get("sessionId")
  if (!sessionId) return NextResponse.json({ error: "Missing sessionId" }, { status: 400 })

  const photosRes = await fetch(`https://photospicker.googleapis.com/v1/sessions/${encodeURIComponent(sessionId)}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${token}` },
  })
  if (!photosRes.ok) return NextResponse.json({ error: `Google Photos Picker delete failed: ${await photosRes.text()}` }, { status: photosRes.status })
  return NextResponse.json({ ok: true })
}
