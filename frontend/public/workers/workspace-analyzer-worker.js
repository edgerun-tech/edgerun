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
])

const CODE_EXTENSIONS = new Set([
  "ts",
  "tsx",
  "js",
  "jsx",
  "rs",
  "go",
  "py",
  "css",
  "html",
  "json",
  "toml",
  "yaml",
  "yml",
  "md",
])

const MAX_ANALYZED_FILES = 400
const MAX_FILE_BYTES = 256 * 1024

function postProgress(phase, completed, total, label) {
  self.postMessage({
    type: "progress",
    phase,
    completed,
    total,
    label,
  })
}

function extension(name) {
  const index = name.lastIndexOf(".")
  return index >= 0 ? name.slice(index + 1).toLowerCase() : ""
}

function isCodeFile(name) {
  return CODE_EXTENSIONS.has(extension(name))
}

function extractImports(content) {
  const imports = new Set()
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

function extractFunctions(content, path) {
  const ext = extension(path)
  const names = new Set()
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

function importMatchesPath(source, targetPath) {
  if (!source.startsWith(".") && !source.startsWith("@/")) return false
  const normalizedSource = source.replace(/^@\//, "").replace(/\.(tsx|ts|jsx|js|json|rs|css)$/, "")
  const normalizedTarget = targetPath.replace(/\.(tsx|ts|jsx|js|json|rs|css)$/, "")
  return normalizedTarget.endsWith(normalizedSource.replace(/^\.\//, "").replace(/^\.\.\//, ""))
}

async function countWork(dir, parentPath = "") {
  let total = 0
  for await (const [, child] of dir.entries()) {
    if (child.kind === "directory") {
      if (IGNORED_DIRS.has(child.name)) continue
      total += 1
      total += await countWork(child, parentPath ? `${parentPath}/${child.name}` : child.name)
    } else {
      total += 1
    }
  }
  return total
}

async function analyzeDirectory(dir, total, state, parentPath = "") {
  for await (const [, child] of dir.entries()) {
    const path = parentPath ? `${parentPath}/${child.name}` : child.name

    if (child.kind === "directory") {
      if (IGNORED_DIRS.has(child.name)) continue
      state.completed += 1
      postProgress("analyzing", state.completed, total, path)
      await analyzeDirectory(child, total, state, path)
      continue
    }

    state.completed += 1
    postProgress("analyzing", state.completed, total, path)

    if (state.analyzedFiles >= MAX_ANALYZED_FILES || !isCodeFile(child.name)) continue
    const file = await child.getFile()
    if (file.size > MAX_FILE_BYTES) continue

    const content = await file.text()
    state.metadata[path] = {
      imports: extractImports(content),
      importedBy: [],
      linesOfCode: content.split(/\r\n|\r|\n/).filter((line) => line.trim().length > 0).length,
      functions: extractFunctions(content, path),
    }
    state.analyzedFiles += 1
  }
}

async function compileCodelyzer() {
  postProgress("compiling", 0, 1, "Compiling codelyzer WASM")
  const response = await fetch("/xray-wire/edgerun_codelyzer.wasm")
  if (!response.ok) throw new Error(`codelyzer wasm fetch failed: HTTP ${response.status}`)
  await WebAssembly.compile(await response.arrayBuffer())
  postProgress("compiling", 1, 1, "Codelyzer compiled")
}

self.onmessage = async (event) => {
  if (event.data?.type !== "analyze") return

  try {
    const rootHandle = event.data.rootHandle
    if (!rootHandle) throw new Error("Missing root directory handle")

    await compileCodelyzer()

    postProgress("counting", 0, 1, "Counting files")
    const total = await countWork(rootHandle)
    postProgress("counting", total, total, `${total} entries`)

    const state = {
      completed: 0,
      analyzedFiles: 0,
      metadata: {},
    }
    await analyzeDirectory(rootHandle, total, state)

    const paths = Object.keys(state.metadata)
    for (const sourcePath of paths) {
      for (const imported of state.metadata[sourcePath].imports) {
        const target = paths.find((candidate) => importMatchesPath(imported, candidate))
        if (target) state.metadata[target].importedBy.push(sourcePath)
      }
    }

    self.postMessage({
      type: "done",
      metadata: state.metadata,
      analyzedFiles: state.analyzedFiles,
      total,
    })
  } catch (error) {
    self.postMessage({
      type: "error",
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
