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
    }
  | {
      id: number
      ok: false
      logs: string[]
      error: string
      compileMs: number
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

    const response: CompileResponse = {
      id,
      ok: true,
      logs,
      wasm,
      wasmSize: wasm.byteLength,
      compileMs: performance.now() - started,
    }

    self.postMessage(response, [wasm.buffer])
  } catch (error) {
    const response: CompileResponse = {
      id,
      ok: false,
      logs,
      error: error instanceof Error ? error.message : String(error),
      compileMs: performance.now() - started,
    }
    self.postMessage(response)
  }
}
