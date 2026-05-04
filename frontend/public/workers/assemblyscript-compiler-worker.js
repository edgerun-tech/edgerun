let cachedAsc = null

function normalizePath(path) {
  return path.replace(/^\.\//, "")
}

function stringifyExportResult(value) {
  if (typeof value === "bigint") return value.toString()
  if (value === undefined) return "undefined"
  return String(value)
}

async function loadAssemblyScriptCompiler() {
  if (!cachedAsc) {
    cachedAsc = import("https://cdn.jsdelivr.net/npm/assemblyscript@0.28.2/dist/asc.js").then((mod) => {
      const asc = "main" in mod ? mod : mod.default
      if (!asc?.main) throw new Error("AssemblyScript compiler loaded, but asc.main was not found")
      return asc
    })
  }
  return cachedAsc
}

async function instantiateAndRun(wasm) {
  const started = performance.now()
  const imports = {
    env: {
      abort(message, fileName, line, column) {
        throw new Error(`abort at ${line}:${column} message=${message} file=${fileName}`)
      },
    },
  }

  const { instance } = await WebAssembly.instantiate(wasm, imports)
  const exports = instance.exports
  const exportNames = Object.keys(exports)
  const callable = exports.run
  const runResult = typeof callable === "function" ? stringifyExportResult(callable()) : undefined

  return {
    instantiateMs: performance.now() - started,
    exports: exportNames,
    runResult,
  }
}

self.onmessage = async (event) => {
  const { id, source } = event.data
  const started = performance.now()
  const logs = []

  try {
    const asc = await loadAssemblyScriptCompiler()
    const files = {
      "assembly/index.ts": source,
    }

    logs.push("compiling assembly/index.ts → module.wasm")

    const result = await asc.main(
      [
        "assembly/index.ts",
        "--outFile",
        "module.wasm",
        "--runtime",
        "stub",
        "--optimize",
      ],
      {
        readFile(name) {
          const normalized = normalizePath(name)
          const file = files[normalized] ?? files[name]
          if (file instanceof Uint8Array) return null
          return file ?? null
        },
        writeFile(name, contents) {
          files[normalizePath(name)] = contents
        },
        listFiles(dirname) {
          const normalized = normalizePath(dirname)
          const prefix = normalized.endsWith("/") ? normalized : `${normalized}/`
          return Object.keys(files).filter((name) => name.startsWith(prefix))
        },
        stdout: {
          write(text) {
            const trimmed = text.trimEnd()
            if (trimmed) logs.push(trimmed)
          },
        },
        stderr: {
          write(text) {
            const trimmed = text.trimEnd()
            if (trimmed) logs.push(trimmed)
          },
        },
      }
    )

    if (result.error) {
      throw result.error instanceof Error ? result.error : new Error(String(result.error))
    }

    const wasm = files["module.wasm"]
    if (!(wasm instanceof Uint8Array)) {
      throw new Error("AssemblyScript compiler did not emit module.wasm")
    }

    const compileMs = performance.now() - started
    logs.push(`compiled ${wasm.byteLength} byte wasm module in ${compileMs.toFixed(2)}ms`)

    const run = await instantiateAndRun(wasm)
    logs.push(`instantiated module in ${run.instantiateMs.toFixed(2)}ms`)
    logs.push(`exports: ${run.exports.join(", ") || "none"}`)
    logs.push(run.runResult === undefined ? "no exported run() function found; module compiled successfully" : `run() → ${run.runResult}`)

    self.postMessage({
      id,
      ok: true,
      logs,
      wasm,
      wasmSize: wasm.byteLength,
      compileMs,
      instantiateMs: run.instantiateMs,
      exports: run.exports,
      runResult: run.runResult,
    }, [wasm.buffer])
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      logs,
      error: error instanceof Error ? error.message : String(error),
      compileMs: performance.now() - started,
      instantiateMs: 0,
    })
  }
}
