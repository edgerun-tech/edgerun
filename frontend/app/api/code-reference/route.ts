import { readFile } from "node:fs/promises"
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
  return path.resolve(process.env.CODEX_WORKSPACE || path.resolve(process.cwd(), ".."))
}

function parseTarget(target: string, explicitLine: string | null) {
  const withoutScheme = target.replace(/^file:\/\//, "")
  const lineMatch = withoutScheme.match(/:(\d+)$/)
  const line = Number(explicitLine || lineMatch?.[1] || 0) || null
  const filePath = lineMatch ? withoutScheme.slice(0, -lineMatch[0].length) : withoutScheme

  return { filePath, line }
}

function safeAbsolutePath(root: string, filePath: string) {
  const absolutePath = path.isAbsolute(filePath)
    ? path.resolve(filePath)
    : path.resolve(root, filePath)

  if (absolutePath !== root && !absolutePath.startsWith(`${root}${path.sep}`)) {
    return null
  }

  return absolutePath
}

function languageFor(filePath: string) {
  return EXTENSION_LANGUAGES[path.extname(filePath).toLowerCase()] || "text"
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
    const text = await readFile(absolutePath, "utf8")
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
