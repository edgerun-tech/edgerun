import { NextRequest } from "next/server"
import { cloudflareFetch } from "../cloudflare"

export async function GET(req: NextRequest) {
  const params = new URLSearchParams({
    per_page: "50",
    page: req.nextUrl.searchParams.get("page") || "1",
  })
  const name = req.nextUrl.searchParams.get("name")
  if (name) params.set("name", name)
  return cloudflareFetch(req, `/zones?${params.toString()}`)
}
