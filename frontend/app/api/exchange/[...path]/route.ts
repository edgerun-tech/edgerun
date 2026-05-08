import { NextRequest, NextResponse } from "next/server"

const DEFAULT_EXCHANGE_API_BASE = "http://127.0.0.1:8787"

function exchangeApiBase(): URL {
  const raw = process.env.EDGERUN_EXCHANGE_API_BASE || DEFAULT_EXCHANGE_API_BASE
  const url = new URL(raw)
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("EDGERUN_EXCHANGE_API_BASE must use http or https")
  }
  if (url.username || url.password) {
    throw new Error("EDGERUN_EXCHANGE_API_BASE must not include credentials")
  }
  url.search = ""
  url.hash = ""
  return url
}

function targetPathFromSegments(path: string[]): string {
  if (path.some((segment) => !segment || segment === "." || segment === "..")) {
    throw new Error("Invalid exchange API path")
  }
  return `/${path.map((segment) => encodeURIComponent(segment)).join("/")}`
}

async function proxy(req: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  try {
    const { path } = await context.params
    const url = new URL(targetPathFromSegments(path), exchangeApiBase())
    url.search = req.nextUrl.search

    const method = req.method.toUpperCase()
    const body = method === "GET" || method === "HEAD" ? undefined : await req.text()

    const response = await fetch(url, {
      method,
      headers: {
        "content-type": req.headers.get("content-type") || "application/json",
      },
      body,
      cache: "no-store",
    })

    const text = await response.text()
    return new NextResponse(text, {
      status: response.status,
      headers: {
        "content-type": response.headers.get("content-type") || "application/json",
      },
    })
  } catch (error) {
    console.error("Exchange API proxy failed", error)
    return NextResponse.json({
      error: "exchange api unavailable",
      detail: process.env.NODE_ENV === "production" ? undefined : error instanceof Error ? error.message : String(error),
    }, { status: 503 })
  }
}

export async function GET(req: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  return proxy(req, context)
}

export async function POST(req: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  return proxy(req, context)
}
