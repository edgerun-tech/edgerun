import { openFile, readFileContent, saveFile } from "@/stores/file-system-store"

export type XrayEditKind = "replace" | "overwrite"

export interface XrayEdit {
  kind: XrayEditKind
  path: string
  search?: string
  replace?: string
  content?: string
}

export interface XrayEditResult {
  path: string
  ok: boolean
  error?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined
}

function normalizeEdit(value: unknown): XrayEdit | null {
  if (!isRecord(value)) return null

  const path = asString(value.path)
  if (!path || path.startsWith("/") || path.includes("..")) return null

  const rawKind = asString(value.kind) || asString(value.type)
  const content = asString(value.content)
  const search = asString(value.search)
  const replace = asString(value.replace)

  if ((rawKind === "overwrite" || content !== undefined) && content !== undefined) {
    return { kind: "overwrite", path, content }
  }

  if ((rawKind === "replace" || search !== undefined || replace !== undefined) && search !== undefined && replace !== undefined) {
    return { kind: "replace", path, search, replace }
  }

  return null
}

export function parseXrayEdits(text: string): XrayEdit[] {
  const edits: XrayEdit[] = []
  const fencePattern = /```(?:xray-edit|xray-edits|repo-xray-edit|repo-xray-edits)\s*\n([\s\S]*?)```/gi

  for (const match of text.matchAll(fencePattern)) {
    const raw = match[1]?.trim()
    if (!raw) continue

    try {
      const parsed = JSON.parse(raw)
      const items = Array.isArray(parsed) ? parsed : [parsed]
      for (const item of items) {
        const edit = normalizeEdit(item)
        if (edit) edits.push(edit)
      }
    } catch {
      // Ignore invalid edit blocks. They are displayed as normal assistant text.
    }
  }

  return edits
}

async function applyOverwrite(edit: XrayEdit): Promise<XrayEditResult> {
  if (edit.content === undefined) return { path: edit.path, ok: false, error: "Missing content" }

  const file = await openFile(edit.path)
  if (!file) return { path: edit.path, ok: false, error: "File is not available through current Repo Xray handle" }

  const ok = await saveFile(edit.path, edit.content)
  return ok ? { path: edit.path, ok: true } : { path: edit.path, ok: false, error: "Save failed" }
}

async function applyReplacement(edit: XrayEdit): Promise<XrayEditResult> {
  if (edit.search === undefined || edit.replace === undefined) {
    return { path: edit.path, ok: false, error: "Missing search/replace" }
  }

  const current = await readFileContent(edit.path)
  if (current == null) return { path: edit.path, ok: false, error: "File is not readable through current Repo Xray handle" }

  const first = current.indexOf(edit.search)
  if (first < 0) return { path: edit.path, ok: false, error: "Search text not found" }
  if (current.indexOf(edit.search, first + edit.search.length) >= 0) {
    return { path: edit.path, ok: false, error: "Search text matched more than once; refusing ambiguous edit" }
  }

  const next = current.slice(0, first) + edit.replace + current.slice(first + edit.search.length)
  await openFile(edit.path)
  const ok = await saveFile(edit.path, next)
  return ok ? { path: edit.path, ok: true } : { path: edit.path, ok: false, error: "Save failed" }
}

export async function applyXrayEdit(edit: XrayEdit): Promise<XrayEditResult> {
  try {
    if (edit.kind === "overwrite") return await applyOverwrite(edit)
    return await applyReplacement(edit)
  } catch (error) {
    return {
      path: edit.path,
      ok: false,
      error: error instanceof Error ? error.message : "Unknown edit failure",
    }
  }
}

export async function applyXrayEdits(edits: XrayEdit[]): Promise<XrayEditResult[]> {
  const results: XrayEditResult[] = []
  for (const edit of edits) {
    results.push(await applyXrayEdit(edit))
  }
  return results
}

export function getXrayEditInstruction(): string {
  return `
When proposing Repo Xray edits, use a fenced JSON block with language xray-edit.
Use replace edits when possible; overwrite only when replacing the whole file is truly intended.

Example:
\`\`\`xray-edit
[
  {
    "kind": "replace",
    "path": "src/example.ts",
    "search": "old exact text",
    "replace": "new exact text"
  }
]
\`\`\`

Rules:
- paths must be relative to the opened repo root
- never use absolute paths
- never use .. path traversal
- search text must be exact and unique
- edits are applied only after the user clicks Apply
`
}
