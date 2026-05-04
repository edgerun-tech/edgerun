/**
 * Native assistant API route.
 * Only proxies LLM calls.
 * Does NOT mutate workflows, open folders, scan codebase, or manage files.
 * Browser Repo Xray context is accepted only as bounded client-provided text.
 */

import { generateText } from "ai"
import { createOpenAI } from "@ai-sdk/openai"
import { toolRegistry } from "@/platform/registries/tool-registry"

const opencode = createOpenAI({
  baseURL: "https://opencode.ai/zen/v1",
  apiKey: process.env.OPENCODE_API_KEY || "",
})

const FREE_MODEL = "minimax-m2.5-free"
const MAX_REPO_CONTEXT_CHARS = 80_000

export const maxDuration = 60

function boundedText(value: unknown, maxChars: number): string {
  if (typeof value !== "string") return ""
  if (value.length <= maxChars) return value
  return `${value.slice(0, maxChars)}\n\n[repo context truncated by assistant API route]`
}

export async function POST(req: Request) {
  const body = await req.json()
  const { messages, systemPrompt } = body
  const repoContext = boundedText(body.repoContext, MAX_REPO_CONTEXT_CHARS)

  if (!systemPrompt) {
    return Response.json({ error: "Missing system prompt" }, { status: 400 })
  }

  const fullSystemPrompt = repoContext
    ? `${systemPrompt}\n\n## Browser Repo Xray\n\n${repoContext}\n\nRepo Xray rules: this context came from a user-granted browser directory handle. Do not claim server filesystem access. Treat it as read-only unless the user explicitly saves a file through the editor.`
    : systemPrompt

  try {
    const result = await generateText({
      model: opencode.languageModel(FREE_MODEL),
      messages: [
        { role: "system", content: fullSystemPrompt },
        ...messages,
      ],
    })

    const tools = Array.from(toolRegistry.get().tools.values()).map(t => ({
      toolId: t.toolId,
      name: t.name,
      description: t.description,
      inputSchema: t.inputSchema,
      riskClass: t.riskClass,
      requiresApproval: t.requiresApproval,
    }))

    return Response.json({
      text: result.text,
      finishReason: result.finishReason,
      tools,
    })
  } catch {
    return Response.json(
      { text: "AI unavailable. Check OPENCODE_API_KEY in .env.local" },
      { status: 503 },
    )
  }
}
