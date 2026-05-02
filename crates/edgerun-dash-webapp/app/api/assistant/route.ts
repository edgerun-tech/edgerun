/**
 * New assistant API route.
 * Only proxies LLM calls.
 * Does NOT mutate workflows, open folders, scan codebase, or manage files.
 * Tool calls are returned to the client for approval/execution via the tool registry.
 */

import { generateText } from "ai"
import { createOpenAI } from "@ai-sdk/openai"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { toolRegistry } from "@/platform/registries/tool-registry"

const opencode = createOpenAI({
  baseURL: "https://opencode.ai/zen/v1",
  apiKey: process.env.OPENCODE_API_KEY || "",
})

const FREE_MODEL = "minimax-m2.5-free"

export const maxDuration = 60

export async function POST(req: Request) {
  const body = await req.json()
  const { messages, systemPrompt } = body

  if (!systemPrompt) {
    return Response.json({ error: "Missing system prompt" }, { status: 400 })
  }

  // Only proxy LLM. No workflow/fs/codebase actions here.
  try {
    const result = await generateText({
      model: opencode(FREE_MODEL),
      messages: [
        { role: "system", content: systemPrompt },
        ...messages,
      ],
    })

    // Include available tools in response for client-side tool call handling
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
  } catch (error) {
    return Response.json(
      { text: "AI unavailable. Check OPENCODE_API_KEY in .env.local" },
      { status: 503 },
    )
  }
}
