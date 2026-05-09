import { spawn, type ChildProcessWithoutNullStreams } from "child_process"
import { mkdtemp, readFile, rm } from "fs/promises"
import { tmpdir } from "os"
import path from "path"

export const runtime = "nodejs"
export const maxDuration = 900

type CodexRequest = {
  prompt?: unknown
  resume?: unknown
  stream?: unknown
}

const CODEX_TIMEOUT_MS = Number(process.env.CODEX_TIMEOUT_MS || 15 * 60_000)
const CODEX_HEALTH_TIMEOUT_MS = 5_000
const OUTPUT_LIMIT = 12000
const encoder = new TextEncoder()

function codexBinary(): string {
  return process.env.CODEX_BINARY || "codex"
}

function workspaceRoot(): string {
  return process.env.CODEX_WORKSPACE || path.resolve(process.cwd(), "..")
}

function codexPolicyArgs(): string[] {
  if (process.env.CODEX_DANGEROUS_BYPASS === "0" || process.env.CODEX_YOLO === "0") return []
  return ["--dangerously-bypass-approvals-and-sandbox"]
}

function appendLimited(current: string, chunk: Buffer): string {
  return (current + chunk.toString()).slice(-OUTPUT_LIMIT)
}

function codexExecArgs(options: {
  resume: boolean
  json?: boolean
  outputFile: string
  workDir: string
}): string[] {
  const args = ["exec", ...codexPolicyArgs(), "-C", options.workDir, "-o", options.outputFile]
  if (options.json) args.push("--json")
  if (options.resume) return [...args, "resume", "--last", "-"]
  return [...args, "-"]
}

function sse(event: string, data: unknown): Uint8Array {
  return encoder.encode(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
}

async function checkCodex(): Promise<{ ok: boolean; version?: string; error?: string }> {
  return await new Promise((resolve) => {
    let settled = false
    let stdout = ""
    let stderr = ""

    const finish = (result: { ok: boolean; version?: string; error?: string }) => {
      if (settled) return
      settled = true
      clearTimeout(timeout)
      resolve(result)
    }

    const proc = spawn(codexBinary(), ["--version"], {
      cwd: workspaceRoot(),
      shell: false,
      env: { ...process.env, CI: "1", NO_COLOR: "1" },
      stdio: ["ignore", "pipe", "pipe"],
    })

    proc.stdout.on("data", (data) => {
      stdout = appendLimited(stdout, data)
    })

    proc.stderr.on("data", (data) => {
      stderr = appendLimited(stderr, data)
    })

    proc.on("close", (code) => {
      finish({
        ok: code === 0,
        version: stdout.trim() || undefined,
        error: code === 0 ? undefined : (stderr.trim() || `codex exited with code ${code}`),
      })
    })

    proc.on("error", (err) => {
      finish({ ok: false, error: err.message })
    })

    const timeout = setTimeout(() => {
      proc.kill()
      finish({ ok: false, error: "codex health check timed out" })
    }, CODEX_HEALTH_TIMEOUT_MS)
  })
}

async function runCodex(resume: boolean, prompt: string, signal?: AbortSignal): Promise<{ ok: boolean; text: string; error?: string }> {
  const workDir = workspaceRoot()
  const tempDir = await mkdtemp(path.join(tmpdir(), "edgerun-codex-"))
  const outputFile = path.join(tempDir, "last-message.txt")
  const procArgs = codexExecArgs({ resume, outputFile, workDir })

  try {
    return await new Promise((resolve) => {
      let settled = false
      let stdout = ""
      let stderr = ""
      let proc: ChildProcessWithoutNullStreams | null = null
      let timeout: ReturnType<typeof setTimeout> | null = null

      const finish = async (result: { ok: boolean; error?: string }) => {
        if (settled) return
        settled = true
        if (timeout) clearTimeout(timeout)
        signal?.removeEventListener("abort", onAbort)

        const fileText = await readFile(outputFile, "utf8").catch(() => "")
        const text = (fileText || stdout || stderr).trim().slice(-OUTPUT_LIMIT)
        resolve({ ok: result.ok, text, error: result.error })
      }

      const onAbort = () => {
        proc?.kill()
        void finish({ ok: false, error: "codex request stopped" })
      }

      if (signal?.aborted) {
        void finish({ ok: false, error: "codex request stopped" })
        return
      }
      signal?.addEventListener("abort", onAbort, { once: true })

      proc = spawn(codexBinary(), procArgs, {
        cwd: workDir,
        shell: false,
        env: { ...process.env, CI: "1", NO_COLOR: "1" },
        stdio: ["pipe", "pipe", "pipe"],
      })

      proc.stdout.on("data", (data) => {
        stdout = appendLimited(stdout, data)
      })

      proc.stderr.on("data", (data) => {
        stderr = appendLimited(stderr, data)
      })

      proc.on("close", (code) => {
        void finish({
          ok: code === 0,
          error: code === 0 ? undefined : (stderr.trim() || `codex exited with code ${code}`),
        })
      })

      proc.on("error", (err) => {
        void finish({ ok: false, error: err.message })
      })

      proc.stdin.end(prompt)

      timeout = setTimeout(() => {
        proc?.kill()
        void finish({ ok: false, error: "codex timed out" })
      }, CODEX_TIMEOUT_MS)
    })
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => undefined)
  }
}

function codexStreamResponse(prompt: string, useResume: boolean, signal?: AbortSignal): Response {
  let currentProc: ChildProcessWithoutNullStreams | null = null
  let closed = false
  let abortHandler: (() => void) | null = null

  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      const emit = (event: string, data: unknown) => {
        if (closed) return
        try {
          controller.enqueue(sse(event, data))
        } catch {
          closed = true
          currentProc?.kill()
        }
      }

      const closeStream = () => {
        if (closed) return
        closed = true
        if (abortHandler) signal?.removeEventListener("abort", abortHandler)
        try {
          controller.close()
        } catch {
          // Client disconnected before the route finished.
        }
      }

      abortHandler = () => {
        closed = true
        currentProc?.kill()
        try {
          controller.error(new DOMException("codex request stopped", "AbortError"))
        } catch {
          // Stream may already be cancelled by the browser.
        }
      }

      if (signal?.aborted) {
        abortHandler()
        return
      }
      signal?.addEventListener("abort", abortHandler, { once: true })

      const runAttempt = async (resume: boolean): Promise<{ ok: boolean; text: string; error?: string }> => {
        if (closed || signal?.aborted) return { ok: false, text: "", error: "codex request stopped" }
        const workDir = workspaceRoot()
        const tempDir = await mkdtemp(path.join(tmpdir(), "edgerun-codex-"))
        const outputFile = path.join(tempDir, "last-message.txt")
        const procArgs = codexExecArgs({ resume, json: true, outputFile, workDir })
        if (closed || signal?.aborted) {
          await rm(tempDir, { recursive: true, force: true }).catch(() => undefined)
          return { ok: false, text: "", error: "codex request stopped" }
        }

        try {
          return await new Promise((resolve) => {
            let settled = false
            let stdout = ""
            let stderr = ""
            let lineBuffer = ""
            let finalText = ""

            const finish = async (result: { ok: boolean; error?: string }) => {
              if (settled) return
              settled = true
              clearTimeout(timeout)

              const fileText = await readFile(outputFile, "utf8").catch(() => "")
              const text = (fileText || finalText || stdout || stderr).trim().slice(-OUTPUT_LIMIT)
              resolve({ ok: result.ok, text, error: result.error })
            }

            const processLine = (line: string) => {
              const trimmed = line.trim()
              if (!trimmed) return
              stdout = (stdout + trimmed + "\n").slice(-OUTPUT_LIMIT)

              try {
                const event = JSON.parse(trimmed) as {
                  type?: string
                  item?: { type?: string; text?: string }
                  message?: string
                }
                emit("codex", event)
                if (event.type === "item.completed" && event.item?.type === "agent_message" && typeof event.item.text === "string") {
                  finalText = event.item.text
                  emit("message", { text: event.item.text })
                } else if (event.type === "turn.started") {
                  emit("status", { text: resume ? "Resuming Codex session..." : "Starting Codex..." })
                } else if (event.type === "turn.completed") {
                  emit("status", { text: "Codex turn completed." })
                }
              } catch {
                emit("delta", { text: line })
              }
            }

            currentProc = spawn(codexBinary(), procArgs, {
              cwd: workDir,
              shell: false,
              env: { ...process.env, CI: "1", NO_COLOR: "1" },
              stdio: ["pipe", "pipe", "pipe"],
            })

            currentProc.stdout.on("data", (data: Buffer) => {
              lineBuffer += data.toString()
              const lines = lineBuffer.split(/\r?\n/)
              lineBuffer = lines.pop() || ""
              for (const line of lines) processLine(line)
            })

            currentProc.stderr.on("data", (data: Buffer) => {
              stderr = appendLimited(stderr, data)
              emit("stderr", { text: data.toString() })
            })

            currentProc.on("close", (code) => {
              if (lineBuffer) processLine(lineBuffer)
              void finish({
                ok: code === 0,
                error: code === 0 ? undefined : (stderr.trim() || `codex exited with code ${code}`),
              })
            })

            currentProc.on("error", (err) => {
              void finish({ ok: false, error: err.message })
            })

            currentProc.stdin.end(prompt)

            const timeout = setTimeout(() => {
              currentProc?.kill()
              void finish({ ok: false, error: "codex timed out" })
            }, CODEX_TIMEOUT_MS)
          })
        } finally {
          await rm(tempDir, { recursive: true, force: true }).catch(() => undefined)
        }
      }

      void (async () => {
        try {
          let result = await runAttempt(useResume)
          if (!result.ok && useResume && /no sessions|not found|could not find|resume/i.test(result.error || "")) {
            emit("status", { text: "No previous Codex session found. Starting a new one..." })
            result = await runAttempt(false)
          }

          if (!result.ok) {
            emit("error", { error: result.error || result.text || "codex failed" })
          } else if (result.text) {
            emit("done", { text: result.text })
          } else {
            emit("done", { text: "Codex completed without a final message." })
          }
        } catch (error) {
          emit("error", { error: error instanceof Error ? error.message : String(error) })
        } finally {
          closeStream()
        }
      })()
    },
    cancel() {
      closed = true
      if (abortHandler) signal?.removeEventListener("abort", abortHandler)
      currentProc?.kill()
    },
  })

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream; charset=utf-8",
      "Cache-Control": "no-cache, no-transform",
      "Connection": "keep-alive",
      "X-Accel-Buffering": "no",
    },
  })
}

export async function GET(): Promise<Response> {
  const health = await checkCodex()
  return Response.json(health, { status: health.ok ? 200 : 503 })
}

export async function POST(req: Request): Promise<Response> {
  const body = await req.json().catch(() => null) as CodexRequest | null
  const prompt = typeof body?.prompt === "string" ? body.prompt.trim() : ""
  if (!prompt) return Response.json({ error: "Missing prompt" }, { status: 400 })

  const useResume = body?.resume !== false
  const wantsStream = req.headers.get("accept")?.includes("text/event-stream") || body?.stream === true
  if (wantsStream) return codexStreamResponse(prompt, useResume, req.signal)

  let result = await runCodex(useResume, prompt, req.signal)
  if (!result.ok && useResume && /no sessions|not found|could not find|resume/i.test(result.error || "")) {
    result = await runCodex(false, prompt, req.signal)
  }

  if (!result.ok) {
    return Response.json({ error: result.error || result.text || "codex failed" }, { status: 500 })
  }

  return Response.json({ text: result.text || "Codex completed without a final message." })
}
