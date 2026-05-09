import { mkdir, readFile, writeFile } from "node:fs/promises"
import path from "node:path"
import { NextRequest, NextResponse } from "next/server"

type BridgeCommand = {
  id: string
  type: string
  payload?: unknown
  createdAt: string
}

type BridgeFile = {
  state: unknown | null
  commands: BridgeCommand[]
}

const BRIDGE_PATH = path.join(process.cwd(), "..", "tmp", "agent-ui-bridge.json")

async function readBridge(): Promise<BridgeFile> {
  try {
    const raw = await readFile(BRIDGE_PATH, "utf8")
    const parsed = JSON.parse(raw) as Partial<BridgeFile>
    return {
      state: parsed.state ?? null,
      commands: Array.isArray(parsed.commands) ? parsed.commands : [],
    }
  } catch {
    return { state: null, commands: [] }
  }
}

async function writeBridge(data: BridgeFile) {
  await mkdir(path.dirname(BRIDGE_PATH), { recursive: true })
  await writeFile(BRIDGE_PATH, `${JSON.stringify(data, null, 2)}\n`, "utf8")
}

export async function GET() {
  const bridge = await readBridge()
  return NextResponse.json(bridge, {
    headers: { "Cache-Control": "no-store" },
  })
}

export async function POST(request: NextRequest) {
  const body = await request.json().catch(() => ({})) as {
    state?: unknown
    command?: Partial<BridgeCommand>
    consumeCommandId?: string
  }
  const bridge = await readBridge()

  if ("state" in body) {
    bridge.state = body.state ?? null
  }

  if (body.command?.type) {
    bridge.commands.push({
      id: body.command.id || `cmd-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      type: body.command.type,
      payload: body.command.payload,
      createdAt: body.command.createdAt || new Date().toISOString(),
    })
  }

  if (body.consumeCommandId) {
    bridge.commands = bridge.commands.filter((command) => command.id !== body.consumeCommandId)
  }

  await writeBridge(bridge)
  return NextResponse.json(bridge, {
    headers: { "Cache-Control": "no-store" },
  })
}
