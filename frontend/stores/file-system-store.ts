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

export interface GitInfo {
  branch: string | null
  head: string | null
  remote: string | null
  isWorktree: boolean
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
  gitInfo: GitInfo | null
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

const GIT_METADATA_FILES = new Set([".git/HEAD", ".git/config", ".git/packed-refs"])

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

function shouldSkipIndexedPath(path: string, kind: FileEntryKind): boolean {
  const parts = path.split("/").filter(Boolean)
  if (parts.some(isIgnoredDirectory)) return true
  const gitIndex = parts.indexOf(".git")
  if (gitIndex === -1) return false
  if (gitIndex === 0 && parts.length === 1) return false
  if (kind === "file" && GIT_METADATA_FILES.has(path)) return false
  return true
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
  gitInfo: null,
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
    const gitInfo = await readGitInfo(handle)
    const hasGit = gitInfo != null

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
      gitInfo,
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

      const gitInfo = await readGitInfo(handle)
      const hasGit = gitInfo != null
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
        gitInfo,
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
        const ignored = child.kind === "directory" && shouldSkipIndexedPath(path, child.kind)
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
          if (shouldSkipIndexedPath(path, child.kind)) continue
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

async function readOptionalFile(dirHandle: FileSystemDirectoryHandle, path: string): Promise<string | null> {
  try {
    const parts = path.split("/").filter(Boolean)
    const filename = parts.pop()
    if (!filename) return null
    let current = dirHandle
    for (const part of parts) current = await current.getDirectoryHandle(part)
    const fileHandle = await current.getFileHandle(filename)
    return await (await fileHandle.getFile()).text()
  } catch {
    return null
  }
}

function parseGitRemote(config: string | null): string | null {
  if (!config) return null
  const lines = config.split(/\r\n|\r|\n/)
  let inOrigin = false
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.startsWith("[remote ")) {
      inOrigin = trimmed.includes('"origin"')
      continue
    }
    if (inOrigin && trimmed.startsWith("url")) {
      return trimmed.split("=").slice(1).join("=").trim() || null
    }
  }
  return null
}

async function readGitInfo(dirHandle: FileSystemDirectoryHandle): Promise<GitInfo | null> {
  const head = await readOptionalFile(dirHandle, ".git/HEAD")
  if (!head) return null

  const trimmedHead = head.trim()
  const refMatch = trimmedHead.match(/^ref:\s+refs\/heads\/(.+)$/)
  const config = await readOptionalFile(dirHandle, ".git/config")

  return {
    branch: refMatch?.[1] ?? null,
    head: refMatch ? null : trimmedHead.slice(0, 12),
    remote: parseGitRemote(config),
    isWorktree: false,
  }
}

function deriveGitInfoFromEntries(entries: FileEntry[], openFiles: OpenFile[]): GitInfo | null {
  const hasGit = entries.some((entry) => entry.path === ".git" && entry.kind === "directory")
  if (!hasGit) return null

  const head = openFiles.find((file) => file.path === ".git/HEAD")?.content.trim() ?? null
  const config = openFiles.find((file) => file.path === ".git/config")?.content ?? null
  const refMatch = head?.match(/^ref:\s+refs\/heads\/(.+)$/)

  return {
    branch: refMatch?.[1] ?? null,
    head: head && !refMatch ? head.slice(0, 12) : null,
    remote: parseGitRemote(config),
    isWorktree: false,
  }
}

async function requestRootWritePermission(): Promise<void> {
  const handle = fileSystemStore.get().rootHandle
  if (!handle) return

  const permission = await (handle as any).queryPermission?.({ mode: "readwrite" })
  if (permission === "granted" || permission === undefined) return

  const requested = await (handle as any).requestPermission?.({ mode: "readwrite" })
  if (requested !== "granted") throw new Error("Write permission denied")
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

function childEntries(path: string) {
  const prefix = `${path}/`
  return fileSystemStore.get().entries.filter((entry) => entry.path === path || entry.path.startsWith(prefix))
}

async function copyFileHandle(source: FileSystemFileHandle, targetDir: FileSystemDirectoryHandle, targetName: string) {
  const file = await source.getFile()
  const target = await targetDir.getFileHandle(targetName, { create: true })
  const writable = await target.createWritable()
  await writable.write(file)
  await writable.close()
}

async function copyDirectoryHandle(source: FileSystemDirectoryHandle, target: FileSystemDirectoryHandle) {
  for await (const [, child] of source.entries()) {
    if (child.kind === "file") {
      await copyFileHandle(child as FileSystemFileHandle, target, child.name)
    } else {
      const targetChild = await target.getDirectoryHandle(child.name, { create: true })
      await copyDirectoryHandle(child as FileSystemDirectoryHandle, targetChild)
    }
  }
}

function renameIndexedEntry(path: string, newName: string) {
  const store = fileSystemStore.get()
  const entry = store.entriesByPath[path]
  if (!entry) return

  const parent = parentOf(path)
  const nextPath = parent ? `${parent}/${newName}` : newName
  const prefix = `${path}/`
  const nextPrefix = `${nextPath}/`

  const entries = store.entries.map((item) => {
    if (item.path !== path && !item.path.startsWith(prefix)) return item
    const renamedPath = item.path === path ? nextPath : item.path.replace(prefix, nextPrefix)
    return {
      ...item,
      name: item.path === path ? newName : item.name,
      path: renamedPath,
      parentPath: item.path === path ? parent : item.parentPath?.replace(prefix.slice(0, -1), nextPath) ?? null,
    }
  })

  const openFiles = store.openFiles.map((file) => {
    if (file.path !== path && !file.path.startsWith(prefix)) return file
    const renamedPath = file.path === path ? nextPath : file.path.replace(prefix, nextPrefix)
    return { ...file, path: renamedPath, name: renamedPath.split("/").pop() ?? file.name }
  })

  fileSystemStore.set({
    ...store,
    entries: sortEntries(entries),
    entriesByPath: makeEntriesByPath(entries),
    openFiles,
    selectedPath: store.selectedPath === path ? nextPath : store.selectedPath?.startsWith(prefix) ? store.selectedPath.replace(prefix, nextPrefix) : store.selectedPath,
    error: null,
  })
}

function deleteIndexedEntry(path: string) {
  const store = fileSystemStore.get()
  const prefix = `${path}/`
  const entries = store.entries.filter((entry) => entry.path !== path && !entry.path.startsWith(prefix))
  const openFiles = store.openFiles.filter((file) => file.path !== path && !file.path.startsWith(prefix))
  const indexedFileCount = entries.filter((entry) => entry.kind === "file").length
  const indexedDirCount = entries.filter((entry) => entry.kind === "directory").length
  const totalBytesIndexed = entries.reduce((total, entry) => total + (entry.kind === "file" ? entry.size ?? 0 : 0), 0)

  fileSystemStore.set({
    ...store,
    entries,
    entriesByPath: makeEntriesByPath(entries),
    openFiles,
    activeFileIndex: openFiles.length ? Math.min(store.activeFileIndex, openFiles.length - 1) : -1,
    indexedFileCount,
    indexedDirCount,
    totalBytesIndexed,
    selectedPath: store.selectedPath === path || store.selectedPath?.startsWith(prefix) ? null : store.selectedPath,
    error: null,
  })
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

export async function deleteEntry(path: string): Promise<boolean> {
  const store = fileSystemStore.get()
  const entry = store.entriesByPath[path]
  if (!entry) return false

  try {
    if (store.rootHandle) {
      await requestRootWritePermission()
      const parent = parentOf(path)
      const dir = parent ? await getDirectoryHandleByPath(parent) : store.rootHandle
      await dir.removeEntry(entry.name, { recursive: entry.kind === "directory" })
      await refreshEntries()
    } else {
      deleteIndexedEntry(path)
    }
    return true
  } catch (err) {
    const error = err instanceof Error ? err.message : "Failed to delete entry"
    fileSystemStore.set({ ...fileSystemStore.get(), error })
    return false
  }
}

export async function renameEntry(path: string, newName: string): Promise<boolean> {
  const name = newName.trim()
  const store = fileSystemStore.get()
  const entry = store.entriesByPath[path]
  if (!entry || !name || name.includes("/")) return false

  try {
    if (store.rootHandle) {
      await requestRootWritePermission()
      const parent = parentOf(path)
      const parentDir = parent ? await getDirectoryHandleByPath(parent) : store.rootHandle

      if (entry.kind === "file") {
        const source = await getFileHandleByPath(path)
        await copyFileHandle(source, parentDir, name)
      } else {
        const source = await getDirectoryHandleByPath(path)
        const target = await parentDir.getDirectoryHandle(name, { create: true })
        await copyDirectoryHandle(source, target)
      }

      await parentDir.removeEntry(entry.name, { recursive: entry.kind === "directory" })
      await refreshEntries()
    } else {
      renameIndexedEntry(path, name)
    }
    return true
  } catch (err) {
    const error = err instanceof Error ? err.message : "Failed to rename entry"
    fileSystemStore.set({ ...fileSystemStore.get(), error })
    return false
  }
}

export async function compressEntry(path: string): Promise<boolean> {
  const store = fileSystemStore.get()
  const entry = store.entriesByPath[path]
  if (!entry) return false

  try {
    const encoder = new TextEncoder()
    const sourceText = entry.kind === "file" && store.rootHandle
      ? await readFileContent(path) ?? ""
      : JSON.stringify(childEntries(path), null, 2)
    const stream = new Blob([encoder.encode(sourceText)]).stream().pipeThrough(new CompressionStream("gzip"))
    const blob = await new Response(stream).blob()
    const archiveName = `${entry.name}.gz`

    if (store.rootHandle) {
      await requestRootWritePermission()
      const parent = parentOf(path)
      const dir = parent ? await getDirectoryHandleByPath(parent) : store.rootHandle
      const archive = await dir.getFileHandle(archiveName, { create: true })
      const writable = await archive.createWritable()
      await writable.write(blob)
      await writable.close()
      await refreshEntries()
    } else {
      const url = URL.createObjectURL(blob)
      const link = document.createElement("a")
      link.href = url
      link.download = archiveName
      link.click()
      URL.revokeObjectURL(url)
    }
    return true
  } catch (err) {
    const error = err instanceof Error ? err.message : "Failed to compress entry"
    fileSystemStore.set({ ...fileSystemStore.get(), error })
    return false
  }
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
    gitInfo: null,
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
    gitRoot: null,
    isGitRepo: false,
    gitInfo: null,
    entries,
    entriesByPath: makeEntriesByPath(entries),
    selectedPath: openFiles[0]?.path || null,
    repoIndexStatus: "ready",
    error: null,
  })
}

export async function openDirectoryFromFileList(files: FileList): Promise<boolean> {
  if (files.length === 0) return false

  const entriesByPath = new Map<string, FileEntry>()
  const openFiles: OpenFile[] = []
  const gitMetadataFiles: OpenFile[] = []
  let totalBytesIndexed = 0
  let rootName = "Selected Folder"

  for (let i = 0; i < files.length; i++) {
    const file = files[i]
    const relativePath = file.webkitRelativePath || file.name
    const parts = relativePath.split("/").filter(Boolean)
    if (parts.length === 0) continue

    if (i === 0 && parts.length > 1) rootName = parts[0]
    const filePath = parts.length > 1 ? parts.slice(1).join("/") : parts.join("/")
    if (shouldSkipIndexedPath(filePath, "file")) continue

    for (let depth = 0; depth < parts.length - 1; depth++) {
      const dirParts = parts.slice(parts.length > 1 ? 1 : 0, depth + 1)
      if (dirParts.length === 0) continue
      const path = dirParts.join("/")
      if (shouldSkipIndexedPath(path, "directory")) continue
      if (!entriesByPath.has(path)) {
        entriesByPath.set(path, {
          name: dirParts[dirParts.length - 1],
          path,
          kind: "directory",
          depth: dirParts.length - 1,
          parentPath: parentOf(path),
          ignored: isIgnoredDirectory(dirParts[dirParts.length - 1]),
          loaded: true,
        })
      }
    }

    const binary = isBinaryPath(file.name)
    entriesByPath.set(filePath, {
      name: file.name,
      path: filePath,
      kind: "file",
      depth: Math.max(0, filePath.split("/").length - 1),
      parentPath: parentOf(filePath),
      size: file.size,
      modified: file.lastModified,
      binary,
      loaded: true,
    })
    totalBytesIndexed += file.size

    if (!binary && GIT_METADATA_FILES.has(filePath)) {
      gitMetadataFiles.push({
        path: filePath,
        name: file.name,
        content: await file.text(),
        handle: null,
        modified: false,
        language: getLanguageFromFilename(file.name),
        binary,
      })
    } else if (openFiles.length < 8 && !binary) {
      openFiles.push({
        path: filePath,
        name: file.name,
        content: await file.text(),
        handle: null,
        modified: false,
        language: getLanguageFromFilename(file.name),
        binary,
      })
    }
  }

  const entries = sortEntries(Array.from(entriesByPath.values()))
  const indexedFileCount = entries.filter((entry) => entry.kind === "file").length
  const indexedDirCount = entries.filter((entry) => entry.kind === "directory").length
  const hasGit = entries.some((entry) => entry.kind === "directory" && entry.path === ".git")
  const gitInfo = deriveGitInfoFromEntries(entries, [...gitMetadataFiles, ...openFiles])
  const s = fileSystemStore.get()

  fileSystemStore.set({
    ...s,
    rootHandle: null,
    repoRootHandle: null,
    rootPath: rootName,
    repoName: rootName,
    repoIndexStatus: "ready",
    repoIndexError: null,
    isLoading: false,
    entries,
    entriesByPath: makeEntriesByPath(entries),
    openFiles,
    activeFileIndex: openFiles.length ? 0 : -1,
    gitRoot: hasGit ? rootName : null,
    isGitRepo: hasGit,
    gitInfo,
    indexedFileCount,
    indexedDirCount,
    totalBytesIndexed,
    selectedPath: openFiles[0]?.path ?? null,
    expandedDirs: [],
    searchQuery: "",
    lastIndexedAt: Date.now(),
    error: null,
    hasPermission: true,
  })

  return true
}
