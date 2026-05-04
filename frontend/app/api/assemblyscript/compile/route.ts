import { NextRequest, NextResponse } from "next/server"
import ascModule from "assemblyscript/asc"

type AscMainResult = { error?: unknown }

type AscCompiler = {
  main: (args: string[], options: Record<string, unknown>) => Promise<AscMainResult>
}

const asc = ascModule as AscCompiler

function normalizePath(path: string): string {
  return path.replace(/^\.\//, "")
}

function toBase64(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("base64")
}

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

export async function POST(req: NextRequest) {
  const started = performance.now()
  const logs: string[] = []

  try {
    const body = await req.json().catch(() => null)
    const source = typeof body?.source === "string" ? body.source : ""

    if (!source.trim()) {
      return NextResponse.json({ ok: false, error: "source is required", logs }, { status: 400 })
    }

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
      },
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

    return NextResponse.json({
      ok: true,
      logs,
      wasmBase64: toBase64(wasm),
      wasmSize: wasm.byteLength,
      compileMs,
    })
  } catch (error) {
    return NextResponse.json({
      ok: false,
      logs,
      error: error instanceof Error ? error.message : String(error),
      compileMs: performance.now() - started,
    }, { status: 200 })
  }
}
