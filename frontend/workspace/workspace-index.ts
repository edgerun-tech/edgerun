import type {
  DepNode,
  DependencyGraph as DependencyGraphData,
} from "@/components/dependencies/DependencyGraph"
import type { FileEntry } from "@/stores/file-system-store"

export type FileCodeMeta = {
  imports: string[]
  importedBy: string[]
  linesOfCode: number | null
  functions: string[]
}

export type CrateDependency = {
  name: string
  requirement: string
  kind: "normal" | "dev" | "build"
  source: "crates.io" | "workspace" | "path" | "git" | "unknown"
  optional: boolean
  path?: string
}

export type CrateTarget = {
  kind: "lib" | "bin" | "test" | "bench" | "example" | "build"
  name: string
  path: string
}

export type CrateIssue = {
  severity: "critical" | "warning" | "info"
  crate: string
  title: string
  detail: string
  path?: string
}

export type WorkspaceCrate = {
  name: string
  version: string | null
  edition: string | null
  manifestPath: string
  rootPath: string
  publish: boolean | null
  license: string | null
  readme: string | null
  isWorkspaceMember: boolean
  targets: CrateTarget[]
  dependencies: CrateDependency[]
  features: string[]
  metrics: {
    rustFiles: number
    linesOfCode: number
    publicItems: number
    unsafeBlocks: number
    unwraps: number
    panics: number
    todos: number
    tests: number
  }
}

export type WorkspaceCrateAnalysis = {
  crates: WorkspaceCrate[]
  issues: CrateIssue[]
  workspaceMembers: string[]
  packageCount: number
  dependencyCount: number
  pathDependencyCount: number
  gitDependencyCount: number
  unsafeCount: number
  testCount: number
  source: "worker" | "open-files" | "empty"
}

export type CrateRiskSummary = {
  critical: number
  warning: number
  info: number
  score: number
}

export type WorkspaceDistribution = {
  label: string
  count: number
  size: number
}

export type WorkspaceAggregate = {
  totalSize: number
  totalLoc: number
  totalFunctions: number
  measuredCodeFiles: number
  byCategory: WorkspaceDistribution[]
  byType: WorkspaceDistribution[]
}

export type WorkspaceHotspot = {
  entry: FileEntry
  loc: number
  functions: number
  imports: number
  importedBy: number
}

export type WorkspaceSuggestion = {
  title: string
  detail: string
  tone: string
}

export function fileTypeBucket(entry: FileEntry) {
  if (entry.kind === "directory") return "folder"
  const ext = entry.name.split(".").pop()?.toLowerCase() || "file"
  if (["ts", "tsx", "js", "jsx", "rs", "go", "py", "css", "html"].includes(ext)) return "code"
  if (["json", "toml", "yaml", "yml"].includes(ext)) return "config"
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext)) return "image"
  if (["md", "txt", "log"].includes(ext)) return "text"
  if (["zip", "gz", "tar", "rar", "7z"].includes(ext)) return "archive"
  return ext
}

export function extractFunctions(content: string, path: string) {
  const ext = path.split(".").pop()?.toLowerCase()
  const names = new Set<string>()
  const patterns =
    ext === "rs"
      ? [/fn\s+([A-Za-z_][\w]*)\s*\(/g]
      : ext === "py"
        ? [/def\s+([A-Za-z_][\w]*)\s*\(/g, /class\s+([A-Za-z_][\w]*)\s*[:(]/g]
        : [
            /function\s+([A-Za-z_$][\w$]*)\s*\(/g,
            /(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?\(/g,
            /(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?[A-Za-z_$][\w$]*\s*=>/g,
            /export\s+(?:default\s+)?function\s+([A-Za-z_$][\w$]*)\s*\(/g,
          ]

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) names.add(match[1])
  }
  return Array.from(names).slice(0, 24)
}

export function extractImports(content: string) {
  const imports = new Set<string>()
  const patterns = [
    /import\s+(?:type\s+)?(?:[\s\S]*?\s+from\s+)?["']([^"']+)["']/g,
    /export\s+[\s\S]*?\s+from\s+["']([^"']+)["']/g,
    /require\(\s*["']([^"']+)["']\s*\)/g,
    /import\(\s*["']([^"']+)["']\s*\)/g,
    /use\s+([A-Za-z_][\w:]+)/g,
    /mod\s+([A-Za-z_][\w]*)\s*;/g,
  ]

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) imports.add(match[1])
  }
  return Array.from(imports).slice(0, 48)
}

export function importMatchesPath(source: string, targetPath: string) {
  if (!source.startsWith(".") && !source.startsWith("@/")) return false
  const normalizedSource = source.replace(/^@\//, "").replace(/\.(tsx|ts|jsx|js|json|rs|css)$/, "")
  const normalizedTarget = targetPath.replace(/\.(tsx|ts|jsx|js|json|rs|css)$/, "")
  return normalizedTarget.endsWith(normalizedSource.replace(/^\.\//, "").replace(/^\.\.\//, ""))
}

function categoryLabel(bucket: string) {
  const labels: Record<string, string> = {
    folder: "Folders",
    code: "Code",
    config: "Config",
    image: "Images",
    text: "Text",
    archive: "Archives",
  }
  return labels[bucket] ?? bucket.toUpperCase()
}

function fileCategoryBucket(entry: FileEntry) {
  const bucket = fileTypeBucket(entry)
  if (["folder", "code", "config", "image", "text", "archive"].includes(bucket)) return bucket
  return "other"
}

export function segmentColor(index: number) {
  const colors = [
    "oklch(0.72 0.18 230)",
    "oklch(0.75 0.16 145)",
    "oklch(0.78 0.17 72)",
    "oklch(0.72 0.18 315)",
    "oklch(0.72 0.16 25)",
    "oklch(0.70 0.14 275)",
    "oklch(0.72 0.08 250)",
  ]
  return colors[index % colors.length]
}

export function aggregateEntries(
  entries: FileEntry[],
  metadata: Record<string, FileCodeMeta>,
): WorkspaceAggregate {
  const byCategory = new Map<string, WorkspaceDistribution>()
  const byType = new Map<string, WorkspaceDistribution>()
  let totalSize = 0
  let totalLoc = 0
  let totalFunctions = 0
  let measuredCodeFiles = 0

  for (const entry of entries) {
    const size = entry.size ?? 0
    totalSize += size

    const category = fileCategoryBucket(entry)
    const categoryItem = byCategory.get(category) ?? { label: categoryLabel(category), count: 0, size: 0 }
    categoryItem.count++
    categoryItem.size += size
    byCategory.set(category, categoryItem)

    const type = entry.kind === "directory" ? "folder" : entry.name.split(".").pop()?.toLowerCase() || "file"
    const typeItem = byType.get(type) ?? { label: type, count: 0, size: 0 }
    typeItem.count++
    typeItem.size += size
    byType.set(type, typeItem)

    const meta = metadata[entry.path]
    if (meta?.linesOfCode != null) {
      totalLoc += meta.linesOfCode
      totalFunctions += meta.functions.length
      measuredCodeFiles++
    }
  }

  const toSorted = (map: Map<string, WorkspaceDistribution>) =>
    Array.from(map.values()).sort((a, b) => b.size - a.size || b.count - a.count)

  return {
    totalSize,
    totalLoc,
    totalFunctions,
    measuredCodeFiles,
    byCategory: toSorted(byCategory),
    byType: toSorted(byType),
  }
}

export function compactChartData(data: WorkspaceDistribution[], limit = 8) {
  if (data.length <= limit) return data
  const head = data.slice(0, limit - 1)
  const rest = data.slice(limit - 1)
  return [
    ...head,
    {
      label: "Other",
      count: rest.reduce((sum, item) => sum + item.count, 0),
      size: rest.reduce((sum, item) => sum + item.size, 0),
    },
  ]
}

export function codeHotspots(entries: FileEntry[], metadata: Record<string, FileCodeMeta>): WorkspaceHotspot[] {
  return entries
    .filter((entry) => entry.kind === "file")
    .map((entry) => {
      const meta = metadata[entry.path]
      return {
        entry,
        loc: meta?.linesOfCode ?? 0,
        functions: meta?.functions.length ?? 0,
        imports: meta?.imports.length ?? 0,
        importedBy: meta?.importedBy.length ?? 0,
      }
    })
    .sort((a, b) => (b.loc + b.functions * 12 + b.importedBy * 8 + b.imports * 4) - (a.loc + a.functions * 12 + a.importedBy * 8 + a.imports * 4))
    .slice(0, 8)
}

export function managementSuggestions(
  entries: FileEntry[],
  metadata: Record<string, FileCodeMeta>,
  isGitRepo: boolean,
): WorkspaceSuggestion[] {
  const files = entries.filter((entry) => entry.kind === "file")
  const measured = Object.keys(metadata).length
  const suggestions: WorkspaceSuggestion[] = []

  if (measured === 0) {
    suggestions.push({
      title: "Run code analysis",
      detail: "Open a real folder handle so the worker can compile codelyzer and extract code metadata.",
      tone: "border-sky-500/30 bg-sky-500/10",
    })
  }
  if (!isGitRepo) {
    suggestions.push({
      title: "No git metadata",
      detail: "Git metrics need a repository root with a .git directory.",
      tone: "border-amber-500/30 bg-amber-500/10",
    })
  }
  if (files.length > measured && measured > 0) {
    suggestions.push({
      title: "Partial code map",
      detail: `${measured}/${files.length} files have code metadata. Keep heavy indexing in the worker path.`,
      tone: "border-violet-500/30 bg-violet-500/10",
    })
  }
  suggestions.push({
    title: "Use table for inventory",
    detail: "Switch to table when you need full paths, imports, LOC, modified time, and file operations.",
    tone: "border-emerald-500/30 bg-emerald-500/10",
  })

  return suggestions.slice(0, 4)
}

export function emptyCrateAnalysis(): WorkspaceCrateAnalysis {
  return {
    crates: [],
    issues: [],
    workspaceMembers: [],
    packageCount: 0,
    dependencyCount: 0,
    pathDependencyCount: 0,
    gitDependencyCount: 0,
    unsafeCount: 0,
    testCount: 0,
    source: "empty",
  }
}

export function crateRiskSummary(analysis: WorkspaceCrateAnalysis): CrateRiskSummary {
  const critical = analysis.issues.filter((issue) => issue.severity === "critical").length
  const warning = analysis.issues.filter((issue) => issue.severity === "warning").length
  const info = analysis.issues.filter((issue) => issue.severity === "info").length
  const score = Math.max(0, 100 - critical * 22 - warning * 8 - info * 2)
  return { critical, warning, info, score }
}

export function topCratesByRisk(analysis: WorkspaceCrateAnalysis, limit = 6): WorkspaceCrate[] {
  const issueWeight = new Map<string, number>()
  for (const issue of analysis.issues) {
    issueWeight.set(issue.crate, (issueWeight.get(issue.crate) ?? 0) + (issue.severity === "critical" ? 40 : issue.severity === "warning" ? 12 : 3))
  }

  return [...analysis.crates]
    .sort((a, b) => {
      const aRisk = (issueWeight.get(a.name) ?? 0) + a.metrics.unsafeBlocks * 8 + a.metrics.unwraps * 2 + a.dependencies.length
      const bRisk = (issueWeight.get(b.name) ?? 0) + b.metrics.unsafeBlocks * 8 + b.metrics.unwraps * 2 + b.dependencies.length
      return bRisk - aRisk
    })
    .slice(0, limit)
}

function dependencyNameFromPath(path: string) {
  const parts = path.split("/")
  const file = parts.pop() ?? path
  const parent = parts.pop()
  return parent ? `${parent}/${file}` : file
}

function packageNameFromImport(source: string) {
  if (source.startsWith("@/") || source.startsWith(".")) return null
  const parts = source.split("/")
  return source.startsWith("@") ? parts.slice(0, 2).join("/") : parts[0]
}

export function buildWorkspaceDependencyGraph(
  entries: FileEntry[],
  metadata: Record<string, FileCodeMeta>,
): DependencyGraphData {
  const codeEntries = entries.filter((entry) => entry.kind === "file" && metadata[entry.path])
  const codePaths = new Set(codeEntries.map((entry) => entry.path))
  const nodes = new Map<string, DepNode>()
  const illegalEdges: DependencyGraphData["illegalEdges"] = []

  const ensureNode = (name: string, source: DepNode["source"]) => {
    const existing = nodes.get(name)
    if (existing) return existing
    const node: DepNode = {
      name,
      version: source === "workspace" ? "workspace" : "external",
      source,
      isIllegal: false,
      usedBy: [],
      usesIllegalDeps: [],
    }
    nodes.set(name, node)
    return node
  }

  const codeRank = (entry: FileEntry) => {
    const meta = metadata[entry.path]
    return (meta.linesOfCode ?? 0) + meta.functions.length * 12 + meta.importedBy.length * 8 + meta.imports.length * 4
  }

  const selectedEntries = [...codeEntries]
    .sort((a, b) => codeRank(b) - codeRank(a))
    .slice(0, 80)

  for (const entry of selectedEntries) {
    ensureNode(dependencyNameFromPath(entry.path), "workspace")
  }

  for (const entry of selectedEntries) {
    const sourceName = dependencyNameFromPath(entry.path)
    const sourceNode = ensureNode(sourceName, "workspace")
    const meta = metadata[entry.path]

    for (const imported of meta.imports.slice(0, 32)) {
      const target = entries.find((candidate) => codePaths.has(candidate.path) && importMatchesPath(imported, candidate.path))
      if (target) {
        const targetNode = ensureNode(dependencyNameFromPath(target.path), "workspace")
        if (!targetNode.usedBy.includes(sourceName)) targetNode.usedBy.push(sourceName)
        continue
      }

      const packageName = packageNameFromImport(imported)
      if (packageName) {
        const externalNode = ensureNode(packageName, "external")
        if (!externalNode.usedBy.includes(sourceName)) externalNode.usedBy.push(sourceName)
        continue
      }

      if (imported.startsWith(".")) {
        sourceNode.usesIllegalDeps.push(imported)
        illegalEdges.push({
          from: sourceName,
          to: imported,
          reason: "Unresolved relative import in indexed files",
        })
      }
    }
  }

  return { nodes, illegalEdges, source: "real" }
}
