import { spawn } from "child_process"

export const maxDuration = 10

export async function POST(req: Request) {
  const { command } = await req.json()

  if (!command || typeof command !== "string") {
    return Response.json({ error: "No command provided" }, { status: 400 })
  }

  const forbidden = ["rm -rf", "sudo", "chmod 777", "mkfs", "dd if="]
  if (forbidden.some(f => command.includes(f))) {
    return Response.json({ error: "Command not allowed" }, { status: 403 })
  }

  return new Promise((resolve) => {
    const parts = command.split(" ")
    const cmd = parts[0]
    const args = parts.slice(1)

    const proc = spawn(cmd, args, {
      shell: true,
      timeout: 5000,
    })

    let output = ""
    let errorOutput = ""

    proc.stdout.on("data", (data) => {
      output += data.toString()
    })

    proc.stderr.on("data", (data) => {
      errorOutput += data.toString()
    })

    proc.on("close", (code) => {
      const result = code === 0 ? output : (errorOutput || `Exit code: ${code}`)
      resolve(Response.json({ 
        output: result.slice(-5000),
        exitCode: code,
      }))
    })

    proc.on("error", (err) => {
      resolve(Response.json({ 
        output: "",
        error: err.message,
      }))
    })

    setTimeout(() => {
      proc.kill()
      resolve(Response.json({ 
        output: output.slice(-5000) + "\n\n[Timed out after 5 seconds]",
        exitCode: -1,
      }))
    }, 5000)
  })
}