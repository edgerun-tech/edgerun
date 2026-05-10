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
