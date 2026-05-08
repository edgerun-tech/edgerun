import { NextRequest, NextResponse } from "next/server"

export async function GET(req: NextRequest) {
  const token = req.cookies.get("google_drive_access_token")?.value
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  const downloadFileId = req.nextUrl.searchParams.get("downloadFileId")
  if (downloadFileId) {
    try {
      const driveRes = await fetch(`https://www.googleapis.com/drive/v3/files/${encodeURIComponent(downloadFileId)}?alt=media`, {
        headers: { Authorization: `Bearer ${token}` },
      })
      if (!driveRes.ok) {
        if (driveRes.status === 401) {
          return NextResponse.json({ error: "Token expired", needReauth: true }, { status: 401 })
        }
        return NextResponse.json({ error: `Google Drive download failed: ${await driveRes.text()}` }, { status: driveRes.status })
      }
      return new NextResponse(await driveRes.text(), {
        headers: { "Content-Type": driveRes.headers.get("Content-Type") || "application/json" },
      })
    } catch (err) {
      return NextResponse.json({ error: `Failed to download Drive file: ${err}` }, { status: 500 })
    }
  }

  const pageToken = req.nextUrl.searchParams.get("pageToken")
  const folderId = req.nextUrl.searchParams.get("folderId") || "root"
  const query = req.nextUrl.searchParams.get("q")?.trim()
  const escapedFolderId = folderId.replace(/'/g, "\\'")
  const escapedQuery = query?.replace(/'/g, "\\'")
  const filters = [
    "trashed=false",
    `'${escapedFolderId}' in parents`,
    escapedQuery ? `name contains '${escapedQuery}'` : "",
  ].filter(Boolean)
  const params = new URLSearchParams({
    pageSize: "100",
    fields: "nextPageToken,files(id,name,mimeType,modifiedTime,size,webViewLink,webContentLink,iconLink,parents)",
    orderBy: "folder,name_natural",
    q: filters.join(" and "),
  })
  if (pageToken) params.set("pageToken", pageToken)

  try {
    const driveRes = await fetch(`https://www.googleapis.com/drive/v3/files?${params.toString()}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (!driveRes.ok) {
      if (driveRes.status === 401) {
        return NextResponse.json({ error: "Token expired", needReauth: true }, { status: 401 })
      }
      return NextResponse.json({ error: `Google Drive API error: ${await driveRes.text()}` }, { status: driveRes.status })
    }
    return NextResponse.json(await driveRes.json())
  } catch (err) {
    return NextResponse.json({ error: `Failed to fetch Drive files: ${err}` }, { status: 500 })
  }
}

export async function POST(req: NextRequest) {
  const token = req.cookies.get("google_drive_access_token")?.value
  if (!token) return NextResponse.json({ error: "Not authenticated" }, { status: 401 })

  const form = await req.formData().catch(() => null)
  const folderId = String(form?.get("folderId") || "root")
  const action = String(form?.get("action") || "upload")

  try {
    if (action === "create-folder") {
      const name = String(form?.get("name") || "").trim()
      if (!name) return NextResponse.json({ error: "Missing folder name" }, { status: 400 })
      const driveRes = await fetch("https://www.googleapis.com/drive/v3/files?fields=id,name,mimeType,modifiedTime,webViewLink", {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          name,
          mimeType: "application/vnd.google-apps.folder",
          parents: [folderId],
        }),
      })
      if (!driveRes.ok) return NextResponse.json({ error: `Google Drive folder create failed: ${await driveRes.text()}` }, { status: driveRes.status })
      return NextResponse.json(await driveRes.json())
    }

    const file = form?.get("file")
    if (!(file instanceof File)) return NextResponse.json({ error: "Missing upload file" }, { status: 400 })
    const metadata = {
      name: file.name,
      parents: [folderId],
      mimeType: file.type || "application/octet-stream",
    }
    const boundary = `edgerun-${crypto.randomUUID()}`
    const encoder = new TextEncoder()
    const body = new Blob([
      encoder.encode(`--${boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n${JSON.stringify(metadata)}\r\n`),
      encoder.encode(`--${boundary}\r\nContent-Type: ${file.type || "application/octet-stream"}\r\n\r\n`),
      file,
      encoder.encode(`\r\n--${boundary}--\r\n`),
    ])
    const driveRes = await fetch("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,mimeType,modifiedTime,size,webViewLink,webContentLink", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": `multipart/related; boundary=${boundary}`,
      },
      body,
    })
    if (!driveRes.ok) return NextResponse.json({ error: `Google Drive upload failed: ${await driveRes.text()}` }, { status: driveRes.status })
    return NextResponse.json(await driveRes.json())
  } catch (err) {
    return NextResponse.json({ error: `Google Drive write failed: ${err}` }, { status: 500 })
  }
}
