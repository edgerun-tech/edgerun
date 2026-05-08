import { NextRequest, NextResponse } from "next/server"
import { mediatedProviderToken } from "../provider-auth"

export const CLOUDFLARE_API_BASE = "https://api.cloudflare.com/client/v4"

export function cloudflareToken(req: NextRequest): string | null {
  return mediatedProviderToken(req, "cloudflare_api_token")
}

export function cloudflareHeaders(token: string): HeadersInit {
  return {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  }
}

export async function cloudflareFetch(req: NextRequest, path: string, init: RequestInit = {}) {
  const token = cloudflareToken(req)
  if (!token) return NextResponse.json({ error: "Not authenticated", needReauth: true }, { status: 401 })
  const response = await fetch(`${CLOUDFLARE_API_BASE}${path}`, {
    ...init,
    headers: {
      ...cloudflareHeaders(token),
      ...(init.headers ?? {}),
    },
  })
  const data = await response.json().catch(async () => ({ success: false, errors: [{ message: await response.text() }] }))
  if (!response.ok || data?.success === false) {
    if (response.status === 401 || response.status === 403) {
      return NextResponse.json({ error: cloudflareError(data, "Cloudflare token denied"), needReauth: true }, { status: response.status })
    }
    return NextResponse.json({ error: cloudflareError(data, "Cloudflare API error"), details: data }, { status: response.status || 500 })
  }
  return NextResponse.json(data)
}

export function cloudflareError(data: unknown, fallback: string): string {
  const errors = data && typeof data === "object" ? (data as { errors?: Array<{ message?: string }> }).errors : undefined
  const message = errors?.map((error) => error.message).filter(Boolean).join("; ")
  return message || fallback
}
