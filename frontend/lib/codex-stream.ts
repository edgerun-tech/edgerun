export type CodexStreamHandlers = {
  onStatus?: (text: string) => void
  onMessage?: (text: string) => void
}

export type CodexStreamOptions = {
  retries?: number
  retryDelayMs?: number
  signal?: AbortSignal
}

type CodexStreamPayload = {
  text?: string
  error?: string
}

function parseSseChunk(
  chunk: string,
  onEvent: (event: string, data: CodexStreamPayload) => void,
): string {
  const parts = chunk.split(/\n\n/)
  const remainder = parts.pop() || ""

  for (const part of parts) {
    let event = "message"
    const dataLines: string[] = []

    for (const line of part.split(/\r?\n/)) {
      if (line.startsWith("event:")) event = line.slice("event:".length).trim()
      if (line.startsWith("data:")) dataLines.push(line.slice("data:".length).trimStart())
    }

    if (!dataLines.length) continue

    let data: CodexStreamPayload
    try {
      data = JSON.parse(dataLines.join("\n")) as CodexStreamPayload
    } catch {
      data = { text: dataLines.join("\n") }
    }
    onEvent(event, data)
  }

  return remainder
}

function isRetryableNetworkError(error: unknown) {
  if (error instanceof DOMException && error.name === "AbortError") return false
  if (error instanceof TypeError) return true
  if (!(error instanceof Error)) return false
  return /network|fetch|failed to fetch|load failed|connection|econnreset|socket/i.test(error.message)
}

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

export async function sendCodexMessage(
  prompt: string,
  handlers: CodexStreamHandlers = {},
  options: CodexStreamOptions = {},
): Promise<string> {
  const maxAttempts = Math.max(1, (options.retries ?? 2) + 1)
  const retryDelayMs = options.retryDelayMs ?? 700
  let lastError: unknown

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await sendCodexMessageOnce(prompt, handlers, options.signal)
    } catch (error) {
      lastError = error
      if (attempt >= maxAttempts || !isRetryableNetworkError(error)) break
      handlers.onStatus?.(`Network error. Retrying ${attempt}/${maxAttempts - 1}...`)
      await wait(retryDelayMs * attempt)
    }
  }

  throw lastError instanceof Error ? lastError : new Error(String(lastError))
}

async function sendCodexMessageOnce(
  prompt: string,
  handlers: CodexStreamHandlers,
  signal?: AbortSignal,
): Promise<string> {
  const res = await fetch("/api/codex", {
    method: "POST",
    headers: {
      "Accept": "text/event-stream",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ prompt, resume: true, stream: true }),
    signal,
  })

  const contentType = res.headers.get("content-type") || ""
  if (!contentType.includes("text/event-stream") || !res.body) {
    const data = await res.json().catch(() => ({})) as CodexStreamPayload
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`)
    const text = data.text || "Codex completed without a final message."
    handlers.onMessage?.(text)
    return text
  }

  if (!res.ok) {
    const data = await res.json().catch(() => ({})) as CodexStreamPayload
    throw new Error(data.error || `HTTP ${res.status}`)
  }

  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ""
  let finalText = ""
  let deliveredMessage = false

  const onEvent = (event: string, data: CodexStreamPayload) => {
    if (event === "message") {
      if (typeof data.text === "string") {
        finalText = data.text
        deliveredMessage = true
        handlers.onMessage?.(data.text)
      }
    } else if (event === "done") {
      if (typeof data.text === "string") finalText = data.text
      if (!deliveredMessage && finalText) handlers.onMessage?.(finalText)
    } else if (event === "status") {
      if (typeof data.text === "string") handlers.onStatus?.(data.text)
    } else if (event === "error") {
      throw new Error(data.error || "codex failed")
    }
  }

  while (true) {
    const { value, done } = await reader.read()
    if (done) break
    buffer = parseSseChunk(buffer + decoder.decode(value, { stream: true }), onEvent)
  }

  parseSseChunk(buffer + decoder.decode(), onEvent)
  return finalText || "Codex completed without a final message."
}
