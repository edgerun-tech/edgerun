import { atom } from "nanostores"

export type FileEntryKind = "file" | "directory"
export type RepoIndexStatus = "idle" | "indexing" | "ready" | "error"

export interface FileEntry {
  name: string
  path: string
  kind: FileEntryKind
  depth: number
  parentPath: string | null
  size?: number
  modified?: number
  ignored?: boolean
  expanded?: boolean
  loaded?: boolean
  binary?: boolean
  error?: string
}

export interface OpenFile {
  path: string
  name: string
  content: string
  handle: FileSystemFileHandle | null
  modified: boolean
  language: string
  binary?: boolean
}

export interface FileSystemStore {
  rootHandle: FileSystemDirectoryHandle | null
  rootPath: string
  repoName: string | null
  repoRootHandle: FileSystemDirectoryHandle | null
  repoIndexStatus: RepoIndexStatus
  repoIndexError: string | null
  isLoading: boolean
  entries: FileEntry[]
  entriesByPath: Record<string, FileEntry>
  openFiles: OpenFile[]
  activeFileIndex: number
  gitRoot: string | null
  isGitRepo: boolean
  indexedFileCount: number
  indexedDirCount: number
  totalBytesIndexed: number
  selectedPath: string | null
  expandedDirs: string[]
  searchQuery: string
  lastIndexedAt: number | null
  error: string | null
  hasPermission: boolean
}

export interface IndexOptions {
  maxDepth?: number
  maxFiles?: number
  maxDirs?: number
  maxTotalEntries?: number
  yieldEvery?: number
}

export interface RepoContextOptions {
  maxChars?: number
  maxFileChars?: number
  includeOpenFiles?: boolean
  includeSelectedFile?: boolean
  includeManifests?: boolean
  includeTreeSummary?: boolean
}

const DB_NAME = "edgerun-fs-db"
const STORE_NAME = "handles"
const STORAGE_KEY = "edgerun_recent_repos"

const DEFAULT_INDEX_OPTIONS: Required<IndexOptions> = {
  maxDepth: 12,
  maxFiles: 5000,
  maxDirs: 1000,
  maxTotalEntries: 6000,
  yieldEvery: 50,
}

const DEFAULT_CONTEXT_OPTIONS: Required<RepoContextOptions> = {
  maxChars: 80_000,
  maxFileChars: 20_000,
  includeOpenFiles: true,
  includeSelectedFile: true,
  includeManifests: true,
  includeTreeSummary: true,
}

const IGNORED_DIRS = new Set([
  ".git",
  "node_modules",
  "target",
  "dist",
  "build",
  ".next",
  ".turbo",
  ".cache",
  "coverage",
  "vendor",
  "tmp",
  "temp",
  ".venv",
  "venv",
  "__pycache__",
  ".idea",
  ".vscode",
])

const BINARY_EXTENSIONS = new Set([
  "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "gz", "tgz", "xz", "7z", "rar",
  "wasm", "so", "dylib", "dll", "exe", "bin", "sqlite", "db", "lock", "mp4", "mov", "mp3", "wav",
  "ttf", "otf", "woff", "woff2",
])

const MANIFEST_FILES = [
  "README.md",
  "readme.md",
  "package.json",
  "Cargo.toml",
  "go.mod",
  "pyproject.toml",
  "requirements.txt",
]

const LIST_ONLY_MANIFESTS = new Set([
  "bun.lockb",
  "pnpm-lock.yaml",
  "yarn.lock",
  "package-lock.json",
])

let db: IDBDatabase | null = null

async function openDB(): Promise<IDBDatabase> {
  if (db) return db

  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => {
      db = request.result
      resolve(db)
    }

    request.onupgradeneeded = (event) => {
      const database = (event.target as IDBOpenDBRequest).result
      if (!database.objectStoreNames.contains(STORE_NAME)) {
        database.createObjectStore(STORE_NAME, { keyPath: "id" })
      }
    }
  })
}

async function saveHandle(id: string, handle: FileSystemDirectoryHandle) {
  const database = await openDB()
  return new Promise<void>((resolve, reject) => {
    const tx = database.transaction(STORE_NAME, "readwrite")
    const store = tx.objectStore(STORE_NAME)
    const request = store.put({ id, name: handle.name, handle })
    request.onsuccess = () => resolve()
    request.onerror = () => reject(request.error)
  })
}

async function loadHandle(id: string): Promise<FileSystemDirectoryHandle | null> {
  const database = await openDB()

  return new Promise((resolve, reject) => {
    const tx = database.transaction(STORE_NAME, "readonly")
    const store = tx.objectStore(STORE_NAME)
    const request = store.get(id)

    request.onsuccess = async () => {
      const result = request.result
      if (!result) {
        resolve(null)
        return
      }

      try {
        const handle = result.handle as FileSystemDirectoryHandle
        const permission = await (handle as any).queryPermission?.({ mode: "readwrite" })
        if (permission === "granted") {
          resolve(handle)
          return
        }

        const requested = await (handle as any).requestPermission?.({ mode: "readwrite" })
        if (requested === "granted") {
          resolve(handle)
        } else {
          await removeHandle(id)
          resolve(null)
        }
      } catch {
        resolve(null)
      }
    }

    request.onerror = () => reject(request.error)
  })
}

async function removeHandle(id: string) {
  const database = await openDB()
  return new Promise<void>((resolve, reject) => {
    const tx = database.transaction(STORE_NAME, "readwrite")
    const store = tx.objectStore(STORE_NAME)
    const request = store.delete(id)
    request.onsuccess = () => resolve()
    request.onerror = () => reject(request.error)
  })
}

function loadRecentRepos(): string[] {
  if (typeof window === "undefined") return []
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]")
  } catch {
    return []
  }
}

function saveRecentRepos(paths: string[]) {
  if (typeof window === "undefined") return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(paths.slice(0, 5)))
}

function makeEntriesByPath(entries: FileEntry[]): Record<string, FileEntry> {
  return Object.fromEntries(entries.map((entry) => [entry.path, entry]))
}

function sortEntries(entries: FileEntry[]): FileEntry[] {
  return [...entries].sort((a, b) => {
    const aParts = a.path.split("/")
    const bParts = b.path.split("/")
    const max = Math.max(aParts.length, bParts.length)

    for (let i = 0; i < max; i++) {
      if (!aParts[i]) return -1
      if (!bParts[i]) return 1
      if (aParts[i] === bParts[i]) continue
      if (a.parentPath === b.parentPath && a.kind !== b.kind) {
        return a.kind === "directory" ? -1 : 1
      }
      return aParts[i].localeCompare(bParts[i])
    }

    return a.path.localeCompare(b.path)
  })
}

function getExtension(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() || ""
  return ext === name.toLowerCase() ? "" : ext
}

function isBinaryPath(path: string): boolean {
  return BINARY_EXTENSIONS.has(getExtension(path))
}

function isIgnoredDirectory(name: string): boolean {
  return IGNORED_DIRS.has(name)
}

function joinPath(parent: string, name: string): string {
  return parent ? `${parent}/${name}` : name
}

function parentOf(path: string): string | null {
  const parts = path.split("/").filter(Boolean)
  parts.pop()
  return parts.length ? parts.join("/") : null
}

function truncate(text: string, maxChars: number): string {
  if (text.length <= maxChars) return text
  return `${text.slice(0, maxChars)}\n\n[truncated ${text.length - maxChars} chars]`
}

async function yieldToBrowser(): Promise<void> {
  if (typeof requestAnimationFrame === "function") {
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
  } else {
    await new Promise<void>((resolve) => setTimeout(resolve, 0))
  }
}

export const fileSystemStore = atom<FileSystemStore>({
  rootHandle: null,
  rootPath: "",
  repoName: null,
  repoRootHandle: null,
  repoIndexStatus: "idle",
  repoIndexError: null,
  isLoading: false,
  entries: [],
  entriesByPath: {},
  openFiles: [],
  activeFileIndex: -1,
  gitRoot: null,
  isGitRepo: false,
  indexedFileCount: 0,
  indexedDirCount: 0,
  totalBytesIndexed: 0,
  selectedPath: null,
  expandedDirs: [],
  searchQuery: "",
  lastIndexedAt: null,
  error: null,
  hasPermission: false,
})

export function isFileSystemAccessSupported(): boolean {
  return typeof window !== "undefined" && "showDirectoryPicker" in window
}

export async function openDirectory(): Promise<boolean> {
  if (!isFileSystemAccessSupported()) {
    const s = fileSystemStore.get()
    fileSystemStore.set({
      ...s,
      isLoading: false,
      repoIndexStatus: "error",
      repoIndexError: "File System Access API not supported",
      error: "File System Access API not supported. Use Chrome/Edge on desktop, or drag & drop files into the browser.",
    })
    return false
  }

  try {
    fileSystemStore.set({ ...fileSystemStore.get(), isLoading: true, repoIndexStatus: "indexing", repoIndexError: null, error: null })

    const handle = await (window as any).showDirectoryPicker({ mode: "readwrite" }) as FileSystemDirectoryHandle
    const hasGit = await checkForGit(handle)

    fileSystemStore.set({
      ...fileSystemStore.get(),
      rootHandle: handle,
      repoRootHandle: handle,
      rootPath: handle.name,
      repoName: handle.name,
      isLoading: true,
      hasPermission: true,
      gitRoot: hasGit ? handle.name : null,
      isGitRepo: hasGit,
      expandedDirs: [],
      selectedPath: null,
      searchQuery: "",
      error: null,
    })

    const paths = loadRecentRepos()
    if (!paths.includes(handle.name)) saveRecentRepos([handle.name, ...paths])

    try {
      await saveHandle(handle.name, handle)
    } catch (e) {
      console.warn("Could not persist handle:", e)
    }

    await refreshEntries()
    return true
  } catch (err) {
    let errorMessage = err instanceof Error ? err.message : "Failed to open directory"

    if (errorMessage.includes("canceled") || errorMessage.includes("abort")) {
      errorMessage = "No folder selected"
    } else if (errorMessage.includes("permission") || errorMessage.includes("denied")) {
      errorMessage = "Permission denied. Try selecting a different folder or check browser permissions."
    }

    const s = fileSystemStore.get()
    fileSystemStore.set({
      ...s,
      isLoading: false,
      repoIndexStatus: "error",
      repoIndexError: errorMessage,
      error: errorMessage,
      hasPermission: errorMessage.includes("permission") || errorMessage.includes("denied") ? false : s.hasPermission,
    })
    console.error("Failed to open directory:", err)
    return false
  }
}

export async function restoreDirectory(): Promise<boolean> {
  const recent = loadRecentRepos()
  if (recent.length === 0) return false

  for (const name of recent) {
    try {
      const handle = await loadHandle(name)
      if (!handle) continue

      const hasGit = await checkForGit(handle)
      fileSystemStore.set({
        ...fileSystemStore.get(),
        rootHandle: handle,
        repoRootHandle: handle,
        rootPath: name,
        repoName: name,
        isLoading: true,
        repoIndexStatus: "indexing",
        repoIndexError: null,
        hasPermission: true,
        gitRoot: hasGit ? name : null,
        isGitRepo: hasGit,
        error: null,
      })

      await refreshEntries()
      return true
    } catch (e) {
      console.warn("Could not restore", name, e)
      await removeHandle(name)
    }
  }

  return false
}

export async function restoreRecentDirectory(): Promise<boolean> {
  return restoreDirectory()
}

export async function refreshEntries() {
  await indexDirectory()
}

export async function indexDirectory(options: IndexOptions = {}): Promise<void> {
  const initial = fileSystemStore.get()
  if (!initial.rootHandle) return

  const opts = { ...DEFAULT_INDEX_OPTIONS, ...options }
  const entries: FileEntry[] = []
  const expandedDirs = new Set(initial.expandedDirs)
  let indexedFileCount = 0
  let indexedDirCount = 0
  let totalBytesIndexed = 0
  let visited = 0
  let stoppedByLimit = false

  fileSystemStore.set({
    ...initial,
    isLoading: true,
    repoIndexStatus: "indexing",
    repoIndexError: null,
    error: null,
  })

  const visit = async (dir: FileSystemDirectoryHandle, parentPathValue: string, depth: number): Promise<void> => {
    if (depth > opts.maxDepth || stoppedByLimit) return

    const local: FileEntry[] = []

    try {
      for await (const [, child] of dir.entries()) {
        if (stoppedByLimit) break

        const path = joinPath(parentPathValue, child.name)
        const ignored = child.kind === "directory" && isIgnoredDirectory(child.name)
        const entry: FileEntry = {
          name: child.name,
          path,
          kind: child.kind,
          depth,
          parentPath: parentPathValue || null,
          ignored,
          expanded: expandedDirs.has(path),
          loaded: child.kind === "file" || ignored,
          binary: child.kind === "file" ? isBinaryPath(child.name) : false,
        }

        if (child.kind === "file") {
          indexedFileCount++
          try {
            const file = await (child as FileSystemFileHandle).getFile()
            entry.size = file.size
            entry.modified = file.lastModified
            totalBytesIndexed += file.size
          } catch (error) {
            entry.error = error instanceof Error ? error.message : "metadata unavailable"
          }
        } else {
          indexedDirCount++
        }

        local.push(entry)
        visited++

        if (visited % opts.yieldEvery === 0) {
          await yieldToBrowser()
        }

        if (
          indexedFileCount >= opts.maxFiles ||
          indexedDirCount >= opts.maxDirs ||
          entries.length + local.length >= opts.maxTotalEntries
        ) {
          stoppedByLimit = true
          break
        }
      }
    } catch (error) {
      entries.push({
        name: parentPathValue.split("/").pop() || initial.rootPath,
        path: parentPathValue || ".",
        kind: "directory",
        depth,
        parentPath: parentOf(parentPathValue),
        ignored: true,
        loaded: false,
        error: error instanceof Error ? error.message : "permission denied",
      })
      return
    }

    local.sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1
      return a.name.localeCompare(b.name)
    })

    entries.push(...local)

    for (const entry of local) {
      if (entry.kind !== "directory" || entry.ignored || stoppedByLimit) continue
      const childDir = await getDirectoryHandleByPath(entry.path)
      await visit(childDir, entry.path, depth + 1)
    }
  }

  try {
    await visit(initial.rootHandle, "", 0)
    const sorted = sortEntries(entries)
    const s = fileSystemStore.get()
    fileSystemStore.set({
      ...s,
      entries: sorted,
      entriesByPath: makeEntriesByPath(sorted),
      indexedFileCount,
      indexedDirCount,
      totalBytesIndexed,
      isLoading: false,
      repoIndexStatus: stoppedByLimit ? "ready" : "ready",
      repoIndexError: stoppedByLimit ? "Index limit reached; repo tree is partial." : null,
      lastIndexedAt: Date.now(),
      error: null,
    })
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : "Permission denied"
    const s = fileSystemStore.get()
    fileSystemStore.set({
      ...s,
      isLoading: false,
      repoIndexStatus: "error",
      repoIndexError: errorMessage,
      error: errorMessage,
      hasPermission: errorMessage.includes("denied") ? false : s.hasPermission,
    })
  }
}

async function checkForGit(dirHandle: FileSystemDirectoryHandle): Promise<boolean> {
  try {
    await dirHandle.getDirectoryHandle(".git")
    return true
  } catch {
    return false
  }
}

function getLanguageFromFilename(filename: string): string {
  const ext = getExtension(filename)
  const langMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    json: "json",
    md: "markdown",
    css: "css",
    scss: "scss",
    html: "html",
    xml: "xml",
    py: "python",
    rs: "rust",
    go: "go",
    rb: "ruby",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    sql: "sql",
    graphql: "graphql",
    gql: "graphql",
    svg: "svg",
    png: "png",
    jpg: "jpeg",
    jpeg: "jpeg",
    gif: "gif",
    wasm: "wasm",
  }
  return langMap[ext] || "text"
}

export async function getDirectoryHandleByPath(path: string): Promise<FileSystemDirectoryHandle> {
  const store = fileSystemStore.get()
  if (!store.rootHandle) throw new Error("No repo open")
  if (!path || path === ".") return store.rootHandle

  const parts = path.split("/").filter(Boolean)
  let current = store.rootHandle
  for (const part of parts) {
    current = await current.getDirectoryHandle(part)
  }
  return current
}

export async function getFileHandleByPath(path: string): Promise<FileSystemFileHandle> {
  const store = fileSystemStore.get()
  if (!store.rootHandle) throw new Error("No repo open")

  const parts = path.split("/").filter(Boolean)
  if (parts.length === 0) throw new Error("Missing file path")

  const filename = parts.pop()!
  const dir = parts.length ? await getDirectoryHandleByPath(parts.join("/")) : store.rootHandle
  return dir.getFileHandle(filename)
}

export async function loadDirectory(path: string): Promise<void> {
  const store = fileSystemStore.get()
  const expandedDirs = Array.from(new Set([...store.expandedDirs, path]))
  fileSystemStore.set({ ...store, expandedDirs })
}

export async function toggleDirectory(path: string): Promise<void> {
  const store = fileSystemStore.get()
  const expanded = new Set(store.expandedDirs)
  if (expanded.has(path)) expanded.delete(path)
  else expanded.add(path)

  const entries = store.entries.map((entry) => (
    entry.path === path ? { ...entry, expanded: expanded.has(path), loaded: true } : entry
  ))

  fileSystemStore.set({
    ...store,
    expandedDirs: Array.from(expanded),
    entries,
    entriesByPath: makeEntriesByPath(entries),
    selectedPath: path,
  })
}

export async function openFile(path: string): Promise<OpenFile | null> {
  const store = fileSystemStore.get()
  if (!store.rootHandle) return null

  const existing = store.openFiles.find(f => f.path === path)
  if (existing) {
    fileSystemStore.set({ ...store, activeFileIndex: store.openFiles.indexOf(existing), selectedPath: path })
    return existing
  }

  try {
    const entry = store.entriesByPath[path]
    if (entry?.kind === "directory") return null
    if (entry?.binary || isBinaryPath(path)) {
      throw new Error("Binary file preview is disabled")
    }

    const fileHandle = await getFileHandleByPath(path)
    const file = await fileHandle.getFile()
    const content = await file.text()
    const name = path.split("/").pop() || path
    const openFile: OpenFile = {
      path,
      name,
      content,
      handle: fileHandle,
      modified: false,
      language: getLanguageFromFilename(name),
      binary: false,
    }

    const s = fileSystemStore.get()
    fileSystemStore.set({
      ...s,
      openFiles: [...s.openFiles, openFile],
      activeFileIndex: s.openFiles.length,
      selectedPath: path,
      error: null,
    })

    return openFile
  } catch (err) {
    const error = err instanceof Error ? err.message : "Failed to open file"
    console.error("Failed to open file:", err)
    fileSystemStore.set({ ...fileSystemStore.get(), error })
    return null
  }
}

export async function saveFile(indexOrPath: number | string, newContent: string): Promise<boolean> {
  const store = fileSystemStore.get()
  const index = typeof indexOrPath === "number"
    ? indexOrPath
    : store.openFiles.findIndex((file) => file.path === indexOrPath)
  const file = store.openFiles[index]
  if (!file || !file.handle) return false

  try {
    const permission = await (file.handle as any).queryPermission?.({ mode: "readwrite" })
    if (permission !== "granted") {
      const requested = await (file.handle as any).requestPermission?.({ mode: "readwrite" })
      if (requested !== "granted") throw new Error("Write permission denied")
    }

    const writable = await file.handle.createWritable()
    await writable.write(newContent)
    await writable.close()

    const updatedFiles = [...store.openFiles]
    updatedFiles[index] = { ...file, content: newContent, modified: false }

    fileSystemStore.set({ ...store, openFiles: updatedFiles, error: null })
    return true
  } catch (err) {
    const error = err instanceof Error ? err.message : "Failed to save file"
    console.error("Failed to save file:", err)
    fileSystemStore.set({ ...fileSystemStore.get(), error })
    return false
  }
}

export function closeFile(index: number) {
  const store = fileSystemStore.get()
  const files = store.openFiles.filter((_, i) => i !== index)
  const activeIndex = files.length === 0
    ? -1
    : store.activeFileIndex >= index
      ? Math.max(0, store.activeFileIndex - 1)
      : store.activeFileIndex

  fileSystemStore.set({ ...store, openFiles: files, activeFileIndex: activeIndex })
}

export function setActiveFile(index: number) {
  const store = fileSystemStore.get()
  const selectedPath = store.openFiles[index]?.path || store.selectedPath
  fileSystemStore.set({ ...store, activeFileIndex: index, selectedPath })
}

export function getActiveFile(): OpenFile | null {
  const store = fileSystemStore.get()
  return store.openFiles[store.activeFileIndex] || null
}

export function getRecentRepos(): string[] {
  return loadRecentRepos()
}

export async function readFileContent(path: string): Promise<string | null> {
  const handle = await getFileHandleByPath(path)
  const file = await handle.getFile()
  return file.text()
}

export function searchRepo(query: string): FileEntry[] {
  const q = query.trim().toLowerCase()
  const entries = fileSystemStore.get().entries
  if (!q) return entries
  return entries.filter((entry) => entry.path.toLowerCase().includes(q))
}

export function setRepoSearchQuery(query: string) {
  fileSystemStore.set({ ...fileSystemStore.get(), searchQuery: query })
}

export function clearRepo() {
  const s = fileSystemStore.get()
  if (s.repoName) void removeHandle(s.repoName)
  fileSystemStore.set({
    ...s,
    rootHandle: null,
    repoRootHandle: null,
    rootPath: "",
    repoName: null,
    repoIndexStatus: "idle",
    repoIndexError: null,
    isLoading: false,
    entries: [],
    entriesByPath: {},
    openFiles: [],
    activeFileIndex: -1,
    gitRoot: null,
    isGitRepo: false,
    indexedFileCount: 0,
    indexedDirCount: 0,
    totalBytesIndexed: 0,
    selectedPath: null,
    expandedDirs: [],
    searchQuery: "",
    lastIndexedAt: null,
    error: null,
    hasPermission: false,
  })
}

export async function buildRepoContext(options: RepoContextOptions = {}): Promise<string> {
  const opts = { ...DEFAULT_CONTEXT_OPTIONS, ...options }
  const store = fileSystemStore.get()
  if (!store.rootHandle || !store.repoName) return ""

  const sections: string[] = []
  const add = (text: string) => {
    const current = sections.join("\n").length
    if (current >= opts.maxChars) return
    sections.push(truncate(text, Math.max(0, opts.maxChars - current)))
  }

  add(`# Repo Xray Context

repo: ${store.repoName}
git: ${store.isGitRepo}
indexed_files: ${store.indexedFileCount}
indexed_dirs: ${store.indexedDirCount}
selected_file: ${store.selectedPath || "null"}
open_files:
${store.openFiles.map((file) => `- ${file.path}`).join("\n") || "- none"}
`)

  if (opts.includeTreeSummary) {
    const tree = store.entries
      .filter((entry) => !entry.path.includes("/.git/"))
      .slice(0, 600)
      .map((entry) => `${"  ".repeat(entry.depth)}${entry.kind === "directory" ? "📁" : "📄"} ${entry.name}${entry.ignored ? " [ignored]" : ""}${entry.binary ? " [binary]" : ""}`)
      .join("\n")
    add(`\n## Tree Summary\n${tree || "No files indexed"}\n`)
  }

  if (opts.includeManifests) {
    const manifestChunks: string[] = []
    for (const path of MANIFEST_FILES) {
      const entry = store.entriesByPath[path]
      if (!entry || entry.kind !== "file" || entry.binary || entry.ignored) continue
      try {
        const content = await readFileContent(path)
        if (content != null) manifestChunks.push(`### ${path}\n${truncate(content, opts.maxFileChars)}`)
      } catch {
        manifestChunks.push(`### ${path}\n[unreadable]`)
      }
    }
    for (const path of LIST_ONLY_MANIFESTS) {
      if (store.entriesByPath[path]) manifestChunks.push(`### ${path}\n[list-only lockfile; content intentionally omitted]`)
    }
    if (manifestChunks.length) add(`\n## Manifests\n${manifestChunks.join("\n\n")}\n`)
  }

  const selected = store.selectedPath ? store.entriesByPath[store.selectedPath] : null
  if (opts.includeSelectedFile && selected?.kind === "file" && !selected.binary && !selected.ignored) {
    try {
      const content = await readFileContent(selected.path)
      if (content != null) add(`\n## Selected File\n### ${selected.path}\n${truncate(content, opts.maxFileChars)}\n`)
    } catch {
      add(`\n## Selected File\n### ${selected.path}\n[unreadable]\n`)
    }
  }

  if (opts.includeOpenFiles && store.openFiles.length) {
    const chunks = store.openFiles
      .filter((file) => !file.binary)
      .map((file) => `### ${file.path}\n${truncate(file.content, opts.maxFileChars)}`)
    if (chunks.length) add(`\n## Open Files\n${chunks.join("\n\n")}\n`)
  }

  return truncate(sections.join("\n"), opts.maxChars)
}

export function clearError() {
  fileSystemStore.set({ ...fileSystemStore.get(), error: null, repoIndexError: null })
}

interface DroppedEntry {
  name: string
  path: string
  kind: FileEntryKind
  depth: number
  parentPath: string | null
}

export async function handleDroppedItems(items: DataTransferItemList): Promise<{ entries: DroppedEntry[]; rootName: string } | null> {
  const entries: DroppedEntry[] = []
  let rootName = "Dropped Files"

  const processEntry = async (entry: FileSystemEntry, path: string, depth = 0): Promise<void> => {
    const entryPath = joinPath(path, entry.name)
    if (entry.isFile) {
      entries.push({
        name: entry.name,
        path: entryPath,
        kind: "file",
        depth,
        parentPath: path || null,
      })
    } else if (entry.isDirectory) {
      const dir = entry as FileSystemDirectoryEntry
      entries.push({
        name: dir.name,
        path: entryPath,
        kind: "directory",
        depth,
        parentPath: path || null,
      })
      const reader = dir.createReader()
      const readEntries = await new Promise<FileSystemEntry[]>((resolve) => reader.readEntries(resolve))
      for (const childEntry of readEntries) await processEntry(childEntry, entryPath, depth + 1)
    }
  }

  for (let i = 0; i < items.length; i++) {
    const item = items[i]
    const entry = item.webkitGetAsEntry()
    if (entry) {
      if (entry.isDirectory) rootName = entry.name
      await processEntry(entry, "", 0)
    }
  }

  if (entries.length === 0) return null
  entries.sort((a, b) => {
    if (a.kind === b.kind) return a.name.localeCompare(b.name)
    return a.kind === "directory" ? -1 : 1
  })

  return { entries, rootName }
}

export async function openDroppedFiles(files: FileList): Promise<void> {
  const openFiles: OpenFile[] = []

  for (let i = 0; i < files.length; i++) {
    const file = files[i]
    const binary = isBinaryPath(file.name)
    const content = binary ? "[binary file dropped; preview disabled]" : await file.text()
    openFiles.push({
      path: file.name,
      name: file.name,
      content,
      handle: null,
      modified: false,
      language: getLanguageFromFilename(file.name),
      binary,
    })
  }

  const entries: FileEntry[] = openFiles.map((file) => ({
    name: file.name,
    path: file.path,
    kind: "file" as const,
    depth: 0,
    parentPath: null,
    binary: file.binary,
    loaded: true,
  }))

  const s = fileSystemStore.get()
  fileSystemStore.set({
    ...s,
    openFiles: [...s.openFiles, ...openFiles],
    activeFileIndex: s.openFiles.length,
    rootPath: "Dropped Files",
    repoName: "Dropped Files",
    rootHandle: null,
    repoRootHandle: null,
    entries,
    entriesByPath: makeEntriesByPath(entries),
    selectedPath: openFiles[0]?.path || null,
    repoIndexStatus: "ready",
    error: null,
  })
}
