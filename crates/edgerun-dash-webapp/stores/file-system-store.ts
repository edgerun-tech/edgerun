import { atom } from "nanostores"

export interface FileEntry {
  name: string
  path: string
  kind: "file" | "directory"
  size?: number
  modified?: Date
}

export interface OpenFile {
  path: string
  name: string
  content: string
  handle: FileSystemFileHandle
  modified: boolean
  language: string
}

export interface FileSystemStore {
  rootHandle: FileSystemDirectoryHandle | null
  rootPath: string
  isLoading: boolean
  entries: FileEntry[]
  openFiles: OpenFile[]
  activeFileIndex: number
  gitRoot: string | null
  error: string | null
  hasPermission: boolean
}

const DB_NAME = "edgerun-fs-db"
const STORE_NAME = "handles"
const STORAGE_KEY = "edgerun_recent_repos"

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
        const permission = await result.handle.queryPermission()
        if (permission === "granted") {
          resolve(result.handle)
        } else {
          const requested = await result.handle.requestPermission()
          if (requested === "granted") {
            resolve(result.handle)
          } else {
            await removeHandle(id)
            resolve(null)
          }
        }
      } catch {
        resolve(null)
      }
    }
    
    request.onerror = () => {
      reject(request.error)
    }
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

export const fileSystemStore = atom<FileSystemStore>({
  rootHandle: null,
  rootPath: "",
  isLoading: false,
  entries: [],
  openFiles: [],
  activeFileIndex: -1,
  gitRoot: null,
  error: null,
  hasPermission: false,
})

export function isFileSystemAccessSupported(): boolean {
  return typeof window !== "undefined" && "showDirectoryPicker" in window
}

export async function openDirectory(): Promise<boolean> {
  if (!isFileSystemAccessSupported()) {
    fileSystemStore.set(s => ({ 
      ...s, 
      isLoading: false, 
      error: "File System Access API not supported. Use Chrome/Edge on desktop, or drag & drop files into the browser." 
    }))
    return false
  }

  try {
    fileSystemStore.set(s => ({ ...s, isLoading: true, error: null }))
    
    const handle = await window.showDirectoryPicker()
    
    fileSystemStore.set(s => ({
      ...s,
      rootHandle: handle,
      rootPath: handle.name,
      isLoading: true,
      hasPermission: true,
      error: null,
    }))
    
    await refreshEntries()
    
    const paths = loadRecentRepos()
    if (!paths.includes(handle.name)) {
      saveRecentRepos([handle.name, ...paths])
    }
    
    try {
      await saveHandle(handle.name, handle)
    } catch (e) {
      console.warn("Could not persist handle:", e)
    }
    
    const hasGit = await checkForGit(handle)
    fileSystemStore.set(s => ({ ...s, gitRoot: hasGit ? s.rootPath : null }))
    
    return true
  } catch (err) {
    let errorMessage = err instanceof Error ? err.message : "Failed to open directory"
    
    if (errorMessage.includes("canceled") || errorMessage.includes("abort")) {
      errorMessage = "No folder selected"
    } else if (errorMessage.includes("permission") || errorMessage.includes("denied")) {
      errorMessage = "Permission denied. Try selecting a different folder or check browser permissions."
    } else if (!isFileSystemAccessSupported()) {
      errorMessage = "Browser doesn't support File System Access API. Use Chrome/Edge on desktop."
    }
    
    fileSystemStore.set(s => ({ 
      ...s, 
      isLoading: false, 
      error: errorMessage,
      hasPermission: errorMessage.includes("permission") || errorMessage.includes("denied") ? false : s.hasPermission
    }))
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
      
      fileSystemStore.set(s => ({
        ...s,
        rootHandle: handle,
        rootPath: name,
        isLoading: true,
        hasPermission: true,
        error: null,
      }))
      
      await refreshEntries()
      
      const hasGit = await checkForGit(handle)
      fileSystemStore.set(s => ({ ...s, gitRoot: hasGit ? s.rootPath : null }))
      
      return true
    } catch (e) {
      console.warn("Could not restore", name, e)
      await removeHandle(name)
    }
  }
  
  return false
}

export async function refreshEntries() {
  const store = fileSystemStore.get()
  if (!store.rootHandle) return
  
  const entries: FileEntry[] = []
  
  try {
    for await (const entry of store.rootHandle.values()) {
      entries.push({
        name: entry.name,
        path: entry.name,
        kind: entry.kind,
      })
    }
    
    entries.sort((a, b) => {
      if (a.kind === b.kind) return a.name.localeCompare(b.name)
      return a.kind === "directory" ? -1 : 1
    })
    
    fileSystemStore.set(s => ({ ...s, entries, isLoading: false, error: null }))
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : "Permission denied"
    fileSystemStore.set(s => ({ 
      ...s, 
      isLoading: false, 
      error: errorMessage,
      hasPermission: errorMessage.includes("denied") ? false : s.hasPermission
    }))
  }
}

async function checkForGit(dirHandle: FileSystemDirectoryHandle): Promise<boolean> {
  try {
    for await (const entry of dirHandle.values()) {
      if (entry.name === ".git" && entry.kind === "directory") {
        return true
      }
    }
  } catch {
    return false
  }
  return false
}

function getLanguageFromFilename(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase() || ""
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

export async function openFile(path: string): Promise<OpenFile | null> {
  const store = fileSystemStore.get()
  if (!store.rootHandle) return null
  
  const existing = store.openFiles.find(f => f.path === path)
  if (existing) {
    fileSystemStore.set(s => ({ ...s, activeFileIndex: s.openFiles.indexOf(existing) }))
    return existing
  }
  
  try {
    const parts = path.split("/").filter(Boolean)
    let current: FileSystemDirectoryHandle | FileSystemFileHandle = store.rootHandle
    
    for (let i = 0; i < parts.length - 1; i++) {
      current = await (current as FileSystemDirectoryHandle).getDirectoryHandle(parts[i])
    }
    
    const fileHandle = await (current as FileSystemDirectoryHandle).getFileHandle(parts[parts.length - 1])
    const file = await fileHandle.getFile()
    const content = await file.text()
    
    const name = parts[parts.length - 1]
    const openFile: OpenFile = {
      path,
      name,
      content,
      handle: fileHandle,
      modified: false,
      language: getLanguageFromFilename(name),
    }
    
    fileSystemStore.set(s => ({
      ...s,
      openFiles: [...s.openFiles, openFile],
      activeFileIndex: s.openFiles.length,
    }))
    
    return openFile
  } catch (err) {
    console.error("Failed to open file:", err)
    return null
  }
}

export async function saveFile(index: number, newContent: string): Promise<boolean> {
  const store = fileSystemStore.get()
  const file = store.openFiles[index]
  if (!file) return false
  
  try {
    const writable = await file.handle.createWritable()
    await writable.write(newContent)
    await writable.close()
    
    const updatedFiles = [...store.openFiles]
    updatedFiles[index] = { ...file, content: newContent, modified: false }
    
    fileSystemStore.set(s => ({ ...s, openFiles: updatedFiles }))
    return true
  } catch (err) {
    console.error("Failed to save file:", err)
    return false
  }
}

export function closeFile(index: number) {
  const store = fileSystemStore.get()
  const files = store.openFiles.filter((_, i) => i !== index)
  const activeIndex = store.activeFileIndex >= index 
    ? Math.max(0, store.activeFileIndex - 1)
    : store.activeFileIndex
  
  fileSystemStore.set({ ...store, openFiles: files, activeFileIndex: activeIndex })
}

export function setActiveFile(index: number) {
  fileSystemStore.set(s => ({ ...s, activeFileIndex: index }))
}

export function getActiveFile(): OpenFile | null {
  const store = fileSystemStore.get()
  return store.openFiles[store.activeFileIndex] || null
}

export function getRecentRepos(): string[] {
  return loadRecentRepos()
}

export async function readFileContent(path: string): Promise<string | null> {
  const file = await openFile(path)
  return file?.content || null
}

export function clearError() {
  fileSystemStore.set(s => ({ ...s, error: null }))
}

interface DroppedEntry {
  name: string
  path: string
  kind: "file" | "directory"
}

export async function handleDroppedItems(items: DataTransferItemList): Promise<{ entries: DroppedEntry[]; rootName: string } | null> {
  const entries: DroppedEntry[] = []
  let rootName = "Dropped Files"
  
  const processEntry = async (entry: FileSystemEntry, path: string): Promise<void> => {
    if (entry.isFile) {
      const file = entry as FileSystemFile
      entries.push({
        name: file.name,
        path: `${path}/${file.name}`.replace(/^\//, ""),
        kind: "file",
      })
    } else if (entry.isDirectory) {
      const dir = entry as FileSystemDirectory
      const dirPath = `${path}/${dir.name}`.replace(/^\//, "")
      entries.push({
        name: dir.name,
        path: dirPath,
        kind: "directory",
      })
      const reader = dir.createReader()
      const readEntries = await new Promise<FileSystemEntry[]>((resolve) => {
        reader.readEntries(resolve)
      })
      for (const childEntry of readEntries) {
        await processEntry(childEntry, dirPath)
      }
    }
  }
  
  for (let i = 0; i < items.length; i++) {
    const item = items[i]
    const entry = item.webkitGetAsEntry()
    if (entry) {
      if (entry.isDirectory) {
        rootName = entry.name
      }
      await processEntry(entry, "")
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
    const content = await file.text()
    const openFile: OpenFile = {
      path: file.name,
      name: file.name,
      content,
      handle: null as any,
      modified: false,
      language: getLanguageFromFilename(file.name),
    }
    openFiles.push(openFile)
  }
  
  fileSystemStore.set(s => ({
    ...s,
    openFiles: [...s.openFiles, ...openFiles],
    activeFileIndex: s.openFiles.length,
    rootPath: "Dropped Files",
    rootHandle: null,
    entries: openFiles.map(f => ({ name: f.name, path: f.path, kind: "file" as const })),
    error: null,
  }))
}