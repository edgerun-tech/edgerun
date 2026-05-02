import { atom } from "nanostores"
import { fileSystemStore } from "./file-system-store"

export interface CodebaseContext {
  rootPath: string | null
  files: Map<string, FileInfo>
  lastScanned: number
  summary: string
}

export interface FileInfo {
  name: string
  path: string
  type: "file" | "directory"
  size?: number
  extension?: string
}

const STORAGE_KEY = "edgerun_codebase_context"

function loadFromStorage(): CodebaseContext | null {
  if (typeof window === "undefined") return null
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

function saveToStorage(ctx: CodebaseContext) {
  if (typeof window === "undefined") return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(ctx))
}

export const codebaseStore = atom<CodebaseContext>({
  rootPath: null,
  files: new Map(),
  lastScanned: 0,
  summary: "",
})

export async function scanCodebase() {
  const fs = fileSystemStore.get()
  if (!fs.rootHandle) return

  const files = new Map<string, FileInfo>()
  
  const scanDir = async (handle: FileSystemDirectoryHandle, path: string, depth = 0) => {
    if (depth > 3) return // Limit depth
    
    for await (const entry of handle.values()) {
      const fullPath = path ? `${path}/${entry.name}` : entry.name
      
      if (entry.kind === "file") {
        const ext = entry.name.split(".").pop()?.toLowerCase() || ""
        files.set(fullPath, {
          name: entry.name,
          path: fullPath,
          type: "file",
          extension: ext,
        })
      } else {
        files.set(fullPath, {
          name: entry.name,
          path: fullPath,
          type: "directory",
        })
        try {
          await scanDir(entry, fullPath, depth + 1)
        } catch {
          // Skip inaccessible dirs
        }
      }
    }
  }

  await scanDir(fs.rootHandle, "")

  const jsFiles = Array.from(files.values()).filter(f => f.extension === "ts" || f.extension === "tsx")
  const rustFiles = Array.from(files.values()).filter(f => f.extension === "rs")
  const configFiles = Array.from(files.values()).filter(f => ["json", "toml", "yaml", "yml"].includes(f.extension || ""))
  
  const summary = `Project at ${fs.rootPath}: ${files.size} files/dirs. ` +
    `TypeScript: ${jsFiles.length}, Rust: ${rustFiles.length}, Config: ${configFiles.length}.`

  const ctx: CodebaseContext = {
    rootPath: fs.rootPath,
    files,
    lastScanned: Date.now(),
    summary,
  }

  codebaseStore.set(ctx)
  saveToStorage(ctx)
}

export function getCodebaseContext(): string {
  const ctx = codebaseStore.get()
  if (!ctx.rootPath) return ""
  
  const extCounts = new Map<string, number>()
  for (const f of ctx.files.values()) {
    if (f.extension) {
      extCounts.set(f.extension, (extCounts.get(f.extension) || 0) + 1)
    }
  }
  
  const extList = Array.from(extCounts.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, 10)
    .map(([ext, count]) => `${ext}: ${count}`)
    .join(", ")

  return `
## Codebase Context

**Project Root**: ${ctx.rootPath}
**Total Files**: ${ctx.files.size}
**Last Scanned**: ${new Date(ctx.lastScanned).toLocaleString()}

**Extensions**: ${extList}

**Key Files**:
${Array.from(ctx.files.keys()).filter(k => k.includes("src/") || k.includes("lib/")).slice(0, 20).map(f => `- ${f}`).join("\n")}

This is the user's actual codebase they're working with.
`
}