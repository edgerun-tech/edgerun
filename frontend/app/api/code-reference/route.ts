import path from "node:path"
import { NextRequest, NextResponse } from "next/server"

export const runtime = "nodejs"

const EXTENSION_LANGUAGES: Record<string, string> = {
  ".css": "css",
  ".html": "html",
  ".js": "javascript",
  ".jsx": "jsx",
  ".json": "json",
  ".md": "markdown",
  ".mdx": "mdx",
  ".mjs": "javascript",
  ".rs": "rust",
  ".sh": "bash",
  ".sql": "sql",
  ".ts": "typescript",
  ".tsx": "tsx",
  ".yml": "yaml",
  ".yaml": "yaml",
}

function workspaceRoot() {
  if (process.env.CODEX_WORKSPACE) return normalizePath(process.env.CODEX_WORKSPACE)

  const pwd = process.env.PWD || ""
  if (pwd.endsWith("/frontend")) return normalizePath(pwd.slice(0, -"/frontend".length))
  return ".."
}

function normalizePath(filePath: string) {
  const absolute = filePath.startsWith("/")
  const parts: string[] = []

  for (const part of filePath.split("/")) {
    if (!part || part === ".") continue
    if (part === "..") {
      if (parts.length > 0 && parts[parts.length - 1] !== "..") {
        parts.pop()
      } else if (!absolute) {
        parts.push(part)
      }
      continue
    }
    parts.push(part)
  }

  const normalized = parts.join("/")
  if (absolute) return `/${normalized}`
  return normalized || "."
}

function parseTarget(target: string, explicitLine: string | null) {
  const withoutScheme = target.replace(/^file:\/\//, "")
  const lineMatch = withoutScheme.match(/:(\d+)$/)
  const line = Number(explicitLine || lineMatch?.[1] || 0) || null
  const filePath = lineMatch ? withoutScheme.slice(0, -lineMatch[0].length) : withoutScheme

  return { filePath, line }
}

function safeAbsolutePath(root: string, filePath: string) {
  const absolutePath = filePath.startsWith("/")
    ? normalizePath(filePath)
    : normalizePath(`${root}/${filePath}`)

  if (absolutePath !== root && !absolutePath.startsWith(`${root}${path.sep}`)) {
    return null
  }

  return absolutePath
}

function languageFor(filePath: string) {
  return EXTENSION_LANGUAGES[path.extname(filePath).toLowerCase()] || "text"
}

async function readWorkspaceFile(filePath: string) {
  const { readFile } = await import("node:fs/promises")
  return await readFile(filePath, "utf8")
}

export async function GET(request: NextRequest) {
  const target = request.nextUrl.searchParams.get("target")
  if (!target) {
    return NextResponse.json({ error: "Missing target" }, { status: 400 })
  }

  const root = workspaceRoot()
  const { filePath, line } = parseTarget(target, request.nextUrl.searchParams.get("line"))
  const absolutePath = safeAbsolutePath(root, filePath)

  if (!absolutePath) {
    return NextResponse.json({ error: "Reference is outside the workspace" }, { status: 403 })
  }

  try {
    const text = await readWorkspaceFile(absolutePath)
    if (text.includes("\0")) {
      return NextResponse.json({ error: "Binary files cannot be previewed" }, { status: 415 })
    }

    const lines = text.split(/\r?\n/)
    const context = Math.min(200, Math.max(24, Number(request.nextUrl.searchParams.get("context") || 80)))
    const startLine = line ? Math.max(1, line - Math.floor(context / 2)) : 1
    const endLine = line ? Math.min(lines.length, startLine + context - 1) : Math.min(lines.length, context)
    const code = lines.slice(startLine - 1, endLine).join("\n")
    const relativePath = path.relative(root, absolutePath)

    return NextResponse.json({
      path: absolutePath,
      relativePath,
      filename: line ? `${relativePath}:${line}` : relativePath,
      language: languageFor(absolutePath),
      code,
      startLine,
      highlightLine: line,
      totalLines: lines.length,
    })
  } catch (error) {
    return NextResponse.json({
      error: error instanceof Error ? error.message : String(error),
    }, { status: 404 })
  }
}
