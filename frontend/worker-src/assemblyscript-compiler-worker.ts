type CompileRequest = {
  id: number
  source: string
}

type AscCompileResult = {
  binary?: Uint8Array
  text?: string
  stdout?: unknown
  stderr?: unknown
  error?: Error | null
}

type AscCompiler = {
  compileString(source: string, options?: Record<string, unknown>): Promise<AscCompileResult>
}

let compilerPromise: Promise<AscCompiler> | null = null

function postProgress(id: number, message: string) {
  self.postMessage({ id, progress: true, message })
}

async function loadCompiler(id: number): Promise<AscCompiler> {
  if (!compilerPromise) {
    postProgress(id, "loading AssemblyScript compiler into browser worker...")
    compilerPromise = import("assemblyscript/asc").then((mod) => mod.default as AscCompiler)
  }
  const compiler = await compilerPromise
  postProgress(id, "AssemblyScript compiler loaded")
  return compiler
}

function stringifyExportResult(value: unknown): string {
  if (typeof value === "bigint") return value.toString()
  if (value === undefined) return "undefined"
  return String(value)
}

function appendCompilerOutput(logs: string[], value: unknown) {
  if (typeof value !== "string") return
  const trimmed = value.trim()
  if (trimmed.length > 0) logs.push(trimmed)
}

async function instantiateAndRun(wasm: Uint8Array) {
  const started = performance.now()
  const imports = {
    env: {
      abort(message: number, fileName: number, line: number, column: number) {
        throw new Error(`abort at ${line}:${column} message=${message} file=${fileName}`)
      },
    },
  }

  const { instance } = await WebAssembly.instantiate(wasm, imports)
  const exports = instance.exports as Record<string, unknown>
  const exportNames = Object.keys(exports)
  const callable = exports.run
  const runResult = typeof callable === "function" ? stringifyExportResult(callable()) : undefined

  return {
    instantiateMs: performance.now() - started,
    exports: exportNames,
    runResult,
  }
}

self.onmessage = async (event: MessageEvent<CompileRequest>) => {
  const { id, source } = event.data
  const logs: string[] = []

  try {
    const compiler = await loadCompiler(id)
    postProgress(id, "compiling AssemblyScript source in browser...")
    const compileStarted = performance.now()
    const result = await compiler.compileString(source, {
      optimizeLevel: 0,
      shrinkLevel: 0,
      runtime: "stub",
    })
    const compileMs = performance.now() - compileStarted

    appendCompilerOutput(logs, result.stdout)
    appendCompilerOutput(logs, result.stderr)
    if (result.error) throw result.error
    if (!result.binary) throw new Error("AssemblyScript compiler did not emit wasm bytes")

    const wasm = result.binary
    logs.push(`compiled in ${compileMs.toFixed(2)}ms`)

    const run = await instantiateAndRun(wasm)
    logs.push(`instantiated module in ${run.instantiateMs.toFixed(2)}ms`)
    logs.push(`exports: ${run.exports.join(", ") || "none"}`)
    logs.push(run.runResult === undefined ? "no exported run() function found; module compiled successfully" : `run() -> ${run.runResult}`)

    ;(self as unknown as { postMessage: (message: unknown, transfer?: Transferable[]) => void }).postMessage({
      id,
      ok: true,
      logs,
      wasm,
      wasmSize: wasm.byteLength,
      compileMs,
      instantiateMs: run.instantiateMs,
      exports: run.exports,
      runResult: run.runResult,
    }, [wasm.buffer as ArrayBuffer])
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      logs,
      error: error instanceof Error ? error.message : String(error),
      compileMs: 0,
      instantiateMs: 0,
    })
  }
}
