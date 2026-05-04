import { NextRequest, NextResponse } from "next/server"

const DEFAULT_EXCHANGE_API_BASE = "http://127.0.0.1:8787"

function exchangeApiBase() {
  return process.env.EDGERUN_EXCHANGE_API_BASE || DEFAULT_EXCHANGE_API_BASE
}

async function proxy(req: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  const { path } = await context.params
  const targetPath = `/${path.join("/")}`
  const url = new URL(targetPath, exchangeApiBase())
  url.search = req.nextUrl.search

  const method = req.method.toUpperCase()
  const body = method === "GET" || method === "HEAD" ? undefined : await req.text()

  try {
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
    return NextResponse.json({
      error: "exchange api unavailable",
      detail: error instanceof Error ? error.message : String(error),
      target: url.toString(),
    }, { status: 503 })
  }
}

export async function GET(req: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  return proxy(req, context)
}

export async function POST(req: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  return proxy(req, context)
}
