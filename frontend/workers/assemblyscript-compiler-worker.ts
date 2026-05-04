import ascModule from "assemblyscript/asc"

type CompileRequest = {
  id: number
  source: string
}

type CompileResponse =
  | {
      id: number
      ok: true
      logs: string[]
      wasm: Uint8Array
      wasmSize: number
      compileMs: number
      instantiateMs: number
      exports: string[]
      runResult?: string
    }
  | {
      id: number
      ok: false
      logs: string[]
      error: string
      compileMs: number
      instantiateMs: number
    }

type AscMainResult = {
  error?: unknown
}

type AscCompiler = {
  main: (
    args: string[],
    options: Record<string, unknown>
  ) => Promise<AscMainResult>
}

const asc = ascModule as AscCompiler

function normalizePath(path: string): string {
  return path.replace(/^\.\//, "")
}

function stringifyExportResult(value: unknown): string {
  if (typeof value === "bigint") return value.toString()
  if (value === undefined) return "undefined"
  return String(value)
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
  const started = performance.now()
  const logs: string[] = []

  try {
    const files: Record<string, string | Uint8Array> = {
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
        readFile(name: string) {
          const normalized = normalizePath(name)
          const file = files[normalized] ?? files[name]
          if (file instanceof Uint8Array) return null
          return file ?? null
        },
        writeFile(name: string, contents: string | Uint8Array) {
          files[normalizePath(name)] = contents
        },
        listFiles(dirname: string) {
          const normalized = normalizePath(dirname)
          const prefix = normalized.endsWith("/") ? normalized : `${normalized}/`
          return Object.keys(files).filter((name) => name.startsWith(prefix))
        },
        stdout: {
          write(text: string) {
            const trimmed = text.trimEnd()
            if (trimmed) logs.push(trimmed)
          },
        },
        stderr: {
          write(text: string) {
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

    const response: CompileResponse = {
      id,
      ok: true,
      logs,
      wasm,
      wasmSize: wasm.byteLength,
      compileMs,
      instantiateMs: run.instantiateMs,
      exports: run.exports,
      runResult: run.runResult,
    }

    self.postMessage(response, [wasm.buffer])
  } catch (error) {
    const response: CompileResponse = {
      id,
      ok: false,
      logs,
      error: error instanceof Error ? error.message : String(error),
      compileMs: performance.now() - started,
      instantiateMs: 0,
    }
    self.postMessage(response)
  }
}
