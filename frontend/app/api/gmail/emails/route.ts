import { NextRequest, NextResponse } from "next/server"
import { mediatedProviderToken } from "../../provider-auth"

interface GmailMessage {
  id: string
  threadId: string
  snippet: string
  payload?: {
    headers: { name: string; value: string }[]
  }
}

interface GmailListResponse {
  messages?: GmailMessage[]
  nextPageToken?: string
  resultSizeEstimate?: number
}

async function getAccessToken(req: NextRequest): Promise<string | null> {
  let token = mediatedProviderToken(req, "gmail_access_token")

  if (!token) return null

  // In production, implement token refresh with refresh_token
  return token
}

function parseEmailHeaders(headers: { name: string; value: string }[]) {
  const get = (name: string) =>
    headers.find((h) => h.name.toLowerCase() === name.toLowerCase())?.value || ""

  return {
    from: get("from"),
    to: get("to"),
    subject: get("subject"),
    date: get("date"),
  }
}

export async function GET(req: NextRequest) {
  const token = await getAccessToken(req)

  if (!token) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 })
  }

  const maxResults = req.nextUrl.searchParams.get("maxResults") || "20"
  const pageToken = req.nextUrl.searchParams.get("pageToken")

  try {
    // List messages
    const listParams = new URLSearchParams({
      maxResults,
      labelIds: "INBOX",
    })
    if (pageToken) listParams.set("pageToken", pageToken)

    const listRes = await fetch(
      `https://gmail.googleapis.com/gmail/v1/users/me/messages?${listParams.toString()}`,
      { headers: { Authorization: `Bearer ${token}` } }
    )

    if (!listRes.ok) {
      if (listRes.status === 401) {
        return NextResponse.json(
          { error: "Token expired", needReauth: true },
          { status: 401 }
        )
      }
      return NextResponse.json(
        { error: `Gmail API error: ${await listRes.text()}` },
        { status: listRes.status }
      )
    }

    const listData: GmailListResponse = await listRes.json()

    if (!listData.messages?.length) {
      return NextResponse.json({ emails: [], nextPageToken: null })
    }

    // Fetch details for each message (batch would be better, but keeping it simple)
    const emails = await Promise.all(
      listData.messages.slice(0, 10).map(async (msg) => {
        const detailRes = await fetch(
          `https://gmail.googleapis.com/gmail/v1/users/me/messages/${msg.id}?format=metadata&metadataHeaders=From&metadataHeaders=To&metadataHeaders=Subject&metadataHeaders=Date`,
          { headers: { Authorization: `Bearer ${token}` } }
        )

        if (!detailRes.ok) return null

        const detail = await detailRes.json()
        const headers = parseEmailHeaders(detail.payload?.headers || [])
        const snippet = detail.snippet || ""

        return {
          id: detail.id,
          threadId: detail.threadId,
          snippet,
          from: headers.from,
          to: headers.to,
          subject: headers.subject || "(no subject)",
          date: headers.date,
          unread: (detail.labelIds || []).includes("UNREAD"),
        }
      })
    )

    return NextResponse.json({
      emails: emails.filter(Boolean),
      nextPageToken: listData.nextPageToken || null,
    })
  } catch (err) {
    return NextResponse.json(
      { error: `Failed to fetch emails: ${err}` },
      { status: 500 }
    )
  }
}
