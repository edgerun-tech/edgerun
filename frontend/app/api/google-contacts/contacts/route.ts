import { NextRequest, NextResponse } from "next/server"
import { mediatedProviderToken } from "../../provider-auth"

export async function GET(req: NextRequest) {
  const token = mediatedProviderToken(req, "google_drive_access_token")
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  const pageToken = req.nextUrl.searchParams.get("pageToken")
  const params = new URLSearchParams({
    personFields: "names,emailAddresses,phoneNumbers,photos,organizations",
    pageSize: "100",
    sortOrder: "LAST_MODIFIED_DESCENDING",
  })
  if (pageToken) params.set("pageToken", pageToken)

  try {
    const peopleRes = await fetch(`https://people.googleapis.com/v1/people/me/connections?${params.toString()}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (!peopleRes.ok) {
      if (peopleRes.status === 401) return NextResponse.json({ error: "Token expired", needReauth: true }, { status: 401 })
      return NextResponse.json({ error: `Google Contacts API error: ${await peopleRes.text()}` }, { status: peopleRes.status })
    }
    return NextResponse.json(await peopleRes.json())
  } catch (err) {
    return NextResponse.json({ error: `Failed to fetch Google Contacts: ${err}` }, { status: 500 })
  }
}
