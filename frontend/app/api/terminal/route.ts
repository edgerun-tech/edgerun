import { spawn } from "child_process"

export const runtime = "nodejs"
export const maxDuration = 10

const DEFAULT_ALLOWED_COMMANDS = ["pwd", "ls", "git", "cargo", "bun", "node", "rg"]
const OUTPUT_LIMIT = 5000
const EXECUTION_TIMEOUT_MS = 5000

function terminalApiEnabled(): boolean {
  return process.env.EDGERUN_ENABLE_TERMINAL_API === "1"
}

function allowedCommands(): Set<string> {
  const configured = process.env.EDGERUN_TERMINAL_COMMANDS
  const commands = configured
    ? configured.split(",").map((command) => command.trim()).filter(Boolean)
    : DEFAULT_ALLOWED_COMMANDS
  return new Set(commands)
}

function authorize(req: Request): Response | null {
  const token = process.env.EDGERUN_TERMINAL_API_TOKEN
  if (process.env.NODE_ENV === "production" && !token) {
    return Response.json({ error: "Terminal API token is required in production" }, { status: 503 })
  }

  if (!token) return null

  const authorization = req.headers.get("authorization")
  if (authorization !== `Bearer ${token}`) {
    return Response.json({ error: "Unauthorized" }, { status: 401 })
  }

  return null
}

function parseCommand(command: string): { cmd: string; args: string[] } | Response {
  const trimmed = command.trim()
  if (!trimmed) {
    return Response.json({ error: "No command provided" }, { status: 400 })
  }

  if (/[;&|`$<>\\\n\r]/.test(trimmed)) {
    return Response.json({ error: "Shell metacharacters are not allowed" }, { status: 403 })
  }

  const [cmd, ...args] = trimmed.split(/\s+/)
  if (!cmd || !allowedCommands().has(cmd)) {
    return Response.json({ error: "Command not allowed" }, { status: 403 })
  }

  return { cmd, args }
}

function appendLimited(current: string, chunk: Buffer): string {
  return (current + chunk.toString()).slice(-OUTPUT_LIMIT)
}

export async function POST(req: Request): Promise<Response> {
  if (!terminalApiEnabled()) {
    return Response.json({ error: "Terminal API disabled" }, { status: 404 })
  }

  const authError = authorize(req)
  if (authError) return authError

  const body = await req.json().catch(() => null) as { command?: unknown } | null
  const command = body?.command

  if (!command || typeof command !== "string") {
    return Response.json({ error: "No command provided" }, { status: 400 })
  }

  const parsed = parseCommand(command)
  if (parsed instanceof Response) return parsed

  return new Promise((resolve) => {
    let settled = false
    const finish = (response: Response) => {
      if (settled) return
      settled = true
      resolve(response)
    }

    const proc = spawn(parsed.cmd, parsed.args, {
      shell: false,
      timeout: EXECUTION_TIMEOUT_MS,
      cwd: process.cwd(),
      env: { ...process.env, CI: "1" },
    })

    let output = ""
    let errorOutput = ""

    proc.stdout.on("data", (data) => {
      output = appendLimited(output, data)
    })

    proc.stderr.on("data", (data) => {
      errorOutput = appendLimited(errorOutput, data)
    })

    proc.on("close", (code) => {
      const result = code === 0 ? output : (errorOutput || `Exit code: ${code}`)
      finish(Response.json({
        output: result.slice(-OUTPUT_LIMIT),
        exitCode: code,
      }))
    })

    proc.on("error", (err) => {
      finish(Response.json({
        output: "",
        error: err.message,
      }))
    })

    setTimeout(() => {
      proc.kill()
      finish(Response.json({
        output: `${output.slice(-OUTPUT_LIMIT)}\n\n[Timed out after 5 seconds]`,
        exitCode: -1,
      }))
    }, EXECUTION_TIMEOUT_MS)
  })
}
