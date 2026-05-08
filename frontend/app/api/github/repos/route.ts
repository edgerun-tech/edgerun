import { NextRequest, NextResponse } from "next/server"
import { githubHeaders } from "../github-headers"
import { mediatedProviderToken } from "../../provider-auth"

export async function GET(req: NextRequest) {
  const token = mediatedProviderToken(req, "github_access_token")
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  const page = req.nextUrl.searchParams.get("page") || "1"
  const params = new URLSearchParams({
    affiliation: "owner,collaborator,organization_member",
    sort: "updated",
    direction: "desc",
    per_page: "50",
    page,
  })

  try {
    const githubRes = await fetch(`https://api.github.com/user/repos?${params.toString()}`, {
      headers: githubHeaders(token),
    })
    if (!githubRes.ok) {
      if (githubRes.status === 401) return NextResponse.json({ error: "Token expired", needReauth: true }, { status: 401 })
      return NextResponse.json({ error: `GitHub API error: ${await githubRes.text()}` }, { status: githubRes.status })
    }
    return NextResponse.json({ repos: await githubRes.json() })
  } catch (err) {
    return NextResponse.json({ error: `Failed to fetch GitHub repositories: ${err}` }, { status: 500 })
  }
}
