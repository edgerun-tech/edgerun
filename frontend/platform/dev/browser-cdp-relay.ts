"use client"

type CdpTarget = {
  id: string
  type: string
  title?: string
  url?: string
  webSocketDebuggerUrl?: string
}

type CdpResponse = {
  id?: number
  result?: unknown
  error?: { message?: string; code?: number; data?: unknown }
  method?: string
  params?: unknown
}

type RelayDestination = "frontend" | "backend" | "chatgpt"
type RelayChatSession = {
  query: string
  label: string
}

type BackendBridge = (message: string, options?: Record<string, unknown>) => unknown | Promise<unknown>
type BrowserTimer = ReturnType<typeof setTimeout>

type ChatMessage = {
  role: string
  text: string
}

type ChatPageState = {
  messages: ChatMessage[]
  userCount: number
  assistantCount: number
  totalCount: number
}

type ChatInputSelection = {
  found: boolean
  selector: string | null
  selectorHint: string
  role: string | null
  isContentEditable: boolean
  isInput: boolean
  tagName: string
  textLabel: string
  score: number
  reason: string
}

type ChatInputPatch = {
  ok: boolean
  selector: string | null
  matchedRole?: string | null
  isContentEditable?: boolean
  isInput?: boolean
  isFocused?: boolean
  isDisabled?: boolean
  beforeText?: string
  afterText?: string
  error?: string
}

export type BrowserCdpRelayState = {
  destination: RelayDestination
  frontend: "ready" | "offline"
  backend: "ready" | "offline"
  backendBridgeRegistered: boolean
  endpoint: string
  chatSession: RelayChatSession
  updatedAtIso: string
}

export type BrowserCdpRelay = {
  endpoint: string
  eventName: typeof FRONTEND_EVENT
  destination: RelayDestination
  setEndpoint(endpoint: string): string
  setDestination(destination: RelayDestination): RelayDestination
  setChatSession(session: string): string
  setBackendBridge(bridge: BackendBridge): void
  status(): BrowserCdpRelayState
  subscribeStatus(handler: (state: BrowserCdpRelayState) => void): () => void
  targets(): Promise<CdpTarget[]>
  connect(query?: string): Promise<BrowserCdpPage>
  connectWebSocket(webSocketUrl: string, target?: Partial<CdpTarget>): Promise<BrowserCdpPage>
  sendToChatGpt(message: string, options?: { target?: string; webSocketUrl?: string; waitMs?: number }): Promise<unknown>
  sendToFrontend(message: string, detail?: Record<string, unknown>): Record<string, unknown>
  subscribe(handler: (message: Record<string, unknown>) => void): () => void
  relay(message: string, destination?: RelayDestination, options?: Record<string, unknown>): Promise<unknown>
}

const DEFAULT_ENDPOINT = "http://127.0.0.1:9222"
const DEFAULT_TIMEOUT_MS = 30_000
const FRONTEND_EVENT = "edgerun:cdp-relay-message"
const FRONTEND_CHANNEL = "edgerun:frontend-relay"
const DESTINATION_STORAGE_KEY = "edgerun:cdp-relay-destination"
const CHAT_SESSION_STORAGE_KEY = "edgerun:cdp-relay-chat-session"

function normalizeEndpoint(endpoint?: string): string {
  const raw = (endpoint || DEFAULT_ENDPOINT).trim()
  if (/^https?:\/\//i.test(raw)) return raw.replace(/\/+$/, "")
  if (/^\d+$/.test(raw)) return `http://127.0.0.1:${raw}`
  if (/^[\w.-]+:\d+$/.test(raw)) return `http://${raw}`
  return raw.replace(/\/+$/, "")
}

function storedDestination(): RelayDestination {
  try {
    const value = window.localStorage.getItem(DESTINATION_STORAGE_KEY)
    if (value === "backend" || value === "frontend" || value === "chatgpt") return value
    return "backend"
  } catch {
    return "backend"
  }
}

function storedChatSession(): RelayChatSession {
  try {
    const value = window.localStorage.getItem(CHAT_SESSION_STORAGE_KEY)
    return parseChatSession(value)
  } catch {
    return { query: "chatgpt.com", label: "chatgpt.com" }
  }
}

function parseChatSession(value: string | null): RelayChatSession {
  const text = (value || "").trim()
  if (!text) return { query: "chatgpt.com", label: "chatgpt.com" }

  const delimiter = text.indexOf("|")
  if (delimiter >= 0) {
    const labelCandidate = text.slice(0, delimiter).trim()
    const queryCandidate = text.slice(delimiter + 1).trim()
    if (queryCandidate) {
      return {
        query: queryCandidate,
        label: labelCandidate || queryCandidate,
      }
    }
  }

  if (text.includes("|")) {
    return {
      query: text.replace(/\|/g, "").trim(),
      label: text.replace(/\|/g, "").trim(),
    }
  }

  return { query: text, label: text }
}

function defaultChatInputScore(input: Element): number {
  const role = (input.getAttribute("role") || "").toLowerCase()
  const ariaLabel = (input.getAttribute("aria-label") || "").toLowerCase()
  const placeholder = (input.getAttribute("placeholder") || "").toLowerCase()
  const tagName = input.tagName.toLowerCase()
  const rect = input.getBoundingClientRect()
  const hasSpace = rect.width > 0 && rect.height > 0
  if (!hasSpace) return -500

  let score = 0
  if (tagName === "textarea") score += 40
  if (input instanceof HTMLInputElement) score += 10
  if (role === "textbox") score += 30
  if (input instanceof HTMLElement && input.isContentEditable) score += 50
  const fieldText = `${ariaLabel} ${placeholder}`
  if (fieldText.includes("prompt") || fieldText.includes("message") || fieldText.includes("chat") || fieldText.includes("ask")) score += 50
  const classText = input.className.toLowerCase()
  if (classText.includes("send") || classText.includes("compose") || classText.includes("prompt") || classText.includes("reply")) score += 10
  if (rect.bottom >= window.innerHeight * 0.45) score += 25
  return score
}

function buildCssSelector(target: Element): string {
  const id = target.getAttribute("id")?.trim()
  if (id) return `#${CSS.escape(id)}`

  const testId = target.getAttribute("data-testid")?.trim() || target.getAttribute("data-test-id")?.trim()
  if (testId) return `[data-testid="${CSS.escape(testId)}"]`

  const name = (target as HTMLInputElement).getAttribute?.("name")?.trim()
  if (name && target.tagName.toLowerCase() === "input") return `input[name="${CSS.escape(name)}"]`

  const parts: string[] = []
  let current: Element | null = target
  let steps = 0
  while (current && current.tagName.toLowerCase() !== "html" && steps < 8) {
    const parent: HTMLElement | null = current.parentElement
    if (!parent) break
    const tag = current.tagName.toLowerCase()
    const siblings = Array.from(parent.children).filter((candidate: Element) => candidate.tagName.toLowerCase() === tag)
    const index = siblings.indexOf(current) + 1
    parts.unshift(index > 1 ? `${tag}:nth-of-type(${index})` : tag)
    current = parent
    steps += 1
  }
  return parts.length ? parts.join(" > ") : target.tagName.toLowerCase()
}

function readChatMessages(): ChatPageState {
  const nodes = Array.from(document.querySelectorAll("[data-message-author-role]"))
  const messages = nodes.map((node) => ({
    role: node.getAttribute("data-message-author-role") || "",
    text: (node.textContent || "").replace(/\s+/g, " ").trim(),
  }))
  return {
    messages,
    totalCount: messages.length,
    userCount: messages.filter((message) => message.role === "user").length,
    assistantCount: messages.filter((message) => message.role === "assistant").length,
  }
}

function findChatComposer(targetHint?: string): ChatInputSelection {
  return findChatComposerForSession(targetHint)
}

function readChatComposer(selector: string): ChatInputPatch {
  const element = document.querySelector(selector)
  if (!(element instanceof Element)) {
    return { ok: false, selector: null, error: "Selected composer disappeared" }
  }

  const matchedRole = element.getAttribute("role")
  if (element instanceof HTMLTextAreaElement || element instanceof HTMLInputElement) {
    return {
      ok: true,
      selector,
      matchedRole,
      isContentEditable: false,
      isInput: true,
      isFocused: document.activeElement === element,
      isDisabled: element.disabled,
      beforeText: String((element as HTMLInputElement).value || ""),
      afterText: String((element as HTMLInputElement).value || ""),
    }
  }

  if (element instanceof HTMLElement && element.isContentEditable) {
    return {
      ok: true,
      selector,
      matchedRole,
      isContentEditable: true,
      isInput: false,
      isFocused: document.activeElement === element,
      isDisabled: element.getAttribute("contenteditable") !== "true" && element.getAttribute("contenteditable") !== "",
      beforeText: (element.textContent || "").trim(),
      afterText: (element.textContent || "").trim(),
    }
  }

  return { ok: false, selector, error: "Selected composer type is not editable" }
}

function setChatComposer(selector: string, text: string): ChatInputPatch {
  const element = document.querySelector(selector)
  if (!(element instanceof Element)) return { ok: false, selector: null, error: "Selected composer disappeared" }

  const safeText = typeof text === "string" ? text : String(text)
  if (element instanceof HTMLTextAreaElement || element instanceof HTMLInputElement) {
    element.focus({ preventScroll: true })
    element.value = safeText
    element.selectionStart = safeText.length
    element.selectionEnd = safeText.length
    element.dispatchEvent(new Event("input", { bubbles: true }))
    element.dispatchEvent(new Event("change", { bubbles: true }))
    return {
      ok: true,
      selector,
      matchedRole: element.getAttribute("role"),
      isInput: true,
      isContentEditable: false,
      isFocused: document.activeElement === element,
      isDisabled: element.disabled,
      beforeText: "",
      afterText: element.value,
    }
  }

  if (element instanceof HTMLElement && element.isContentEditable) {
    element.focus({ preventScroll: true })
    element.textContent = safeText
    element.dispatchEvent(new Event("input", { bubbles: true, composed: true }))
    element.dispatchEvent(new Event("change", { bubbles: true, composed: true }))
    return {
      ok: true,
      selector,
      matchedRole: element.getAttribute("role"),
      isInput: false,
      isContentEditable: true,
      isFocused: document.activeElement === element,
      isDisabled: element.getAttribute("contenteditable") !== "true" && element.getAttribute("contenteditable") !== "",
      beforeText: "",
      afterText: (element.textContent || "").trim(),
    }
  }

  return { ok: false, selector, error: "Selected composer type is not editable" }
}

function scoreTarget(target: CdpTarget, query?: string): number {
  if (!target.webSocketDebuggerUrl) return -1000
  if (target.type !== "page") return -100
  if (!query) return 10
  const q = query.toLowerCase()
  const fields = [target.id, target.title, target.url, target.type].filter(Boolean).map((field) => String(field).toLowerCase())
  let score = 40
  for (const field of fields) {
    if (field === q) score += 100
    if (field.includes(q)) score += field.startsWith(q) ? 50 : 25
  }
  return score
}

async function fetchTargets(endpoint?: string): Promise<CdpTarget[]> {
  const response = await fetch(`${normalizeEndpoint(endpoint)}/json/list`, { cache: "no-store" })
  if (!response.ok) throw new Error(`CDP target list failed: ${response.status}`)
  return response.json() as Promise<CdpTarget[]>
}

async function findTarget(endpoint: string | undefined, query?: string): Promise<CdpTarget> {
  const targets = await fetchTargets(endpoint)
  const ranked = targets
    .map((target) => ({ target, score: scoreTarget(target, query) }))
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score)
  const target = ranked[0]?.target
  if (!target?.webSocketDebuggerUrl) throw new Error(`No CDP page matched ${query || "default page"}`)
  return target
}

class BrowserCdpConnection {
  private ws: WebSocket | null = null
  private nextId = 1
  private openPromise: Promise<void> | null = null
  private readonly pending = new Map<number, {
    method: string
    resolve: (value: unknown) => void
    reject: (error: Error) => void
    timer: BrowserTimer
  }>()

  constructor(
    private readonly webSocketUrl: string,
    private readonly timeoutMs = DEFAULT_TIMEOUT_MS,
  ) {}

  open(): Promise<void> {
    if (this.openPromise) return this.openPromise
    this.openPromise = new Promise((resolve, reject) => {
      const ws = new WebSocket(this.webSocketUrl)
      this.ws = ws
      ws.addEventListener("open", () => resolve(), { once: true })
      ws.addEventListener("error", () => reject(new Error(`CDP websocket failed: ${this.webSocketUrl}`)), { once: true })
      ws.addEventListener("message", (event) => this.handleMessage(event.data))
      ws.addEventListener("close", () => this.closePending())
    })
    return this.openPromise
  }

  async send<T = unknown>(method: string, params: Record<string, unknown> = {}, timeoutMs = this.timeoutMs): Promise<T> {
    await this.open()
    const ws = this.ws
    if (!ws || ws.readyState !== WebSocket.OPEN) throw new Error("CDP websocket is not open")
    const id = this.nextId++
    return await new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new Error(`CDP command timed out: ${method}`))
      }, timeoutMs)
      this.pending.set(id, { method, resolve: resolve as (value: unknown) => void, reject, timer })
      ws.send(JSON.stringify({ id, method, params }))
    })
  }

  close(): void {
    this.ws?.close()
    this.closePending()
  }

  private handleMessage(raw: unknown): void {
    const text = typeof raw === "string" ? raw : String(raw)
    const payload = JSON.parse(text) as CdpResponse
    if (!payload.id) return
    const pending = this.pending.get(payload.id)
    if (!pending) return
    clearTimeout(pending.timer)
    this.pending.delete(payload.id)
    if (payload.error) pending.reject(new Error(`CDP ${pending.method} failed: ${payload.error.message || payload.error.code}`))
    else pending.resolve(payload.result)
  }

  private closePending(): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer)
      pending.reject(new Error(`CDP websocket closed while waiting for ${pending.method}`))
    }
    this.pending.clear()
  }
}

class BrowserCdpPage {
  constructor(
    private readonly connection: BrowserCdpConnection,
    readonly target: CdpTarget,
  ) {}

  async evaluate<T = unknown>(expression: string): Promise<T> {
    const result = await this.connection.send<{ result?: { value?: T }, exceptionDetails?: unknown }>("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
      includeCommandLineAPI: true,
    })
    if (result.exceptionDetails) throw new Error(`Runtime.evaluate threw: ${JSON.stringify(result.exceptionDetails)}`)
    return result.result?.value as T
  }

  async focus(locator = "role=textbox"): Promise<unknown> {
    return await this.evaluate(`(${focusElementInPage.toString()})(${JSON.stringify(locator)})`)
  }

  async type(text: string, locator = "role=textbox"): Promise<void> {
    await this.focus(locator)
    await this.connection.send("Input.insertText", { text })
  }

  async press(key = "Enter"): Promise<void> {
    const code = key.length === 1 ? `Key${key.toUpperCase()}` : key
    const keyCode = key.length === 1 ? key.toUpperCase().charCodeAt(0) : 0
    await this.connection.send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode: keyCode, nativeVirtualKeyCode: keyCode })
    await this.connection.send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: keyCode, nativeVirtualKeyCode: keyCode })
  }

  close(): void {
    this.connection.close()
  }
}

function focusElementInPage(locator: string) {
  const textOf = (el: Element) => [
    el.getAttribute("aria-label"),
    el.getAttribute("placeholder"),
    el.getAttribute("data-placeholder"),
    el.textContent,
  ].filter(Boolean).join(" ").trim()
  const visible = (el: Element) => {
    const rect = el.getBoundingClientRect()
    const style = getComputedStyle(el)
    return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden"
  }
  let candidates: Element[]
  if (locator.startsWith("role=")) candidates = Array.from(document.querySelectorAll(`[role="${CSS.escape(locator.slice(5))}"]`))
  else if (locator.startsWith("text=")) candidates = Array.from(document.querySelectorAll("button,a,input,textarea,[role=textbox],[contenteditable=true],div,span")).filter((el) => textOf(el).toLowerCase().includes(locator.slice(5).toLowerCase()))
  else if (locator.startsWith("placeholder=")) candidates = Array.from(document.querySelectorAll("input,textarea,[role=textbox],[contenteditable=true]")).filter((el) => textOf(el).toLowerCase().includes(locator.slice(12).toLowerCase()))
  else candidates = Array.from(document.querySelectorAll(locator))
  const el = candidates.find(visible) as HTMLElement | undefined
  if (!el) return { found: false }
  el.scrollIntoView({ block: "center", inline: "center" })
  el.focus({ preventScroll: true })
  if (el.isContentEditable) {
    const range = document.createRange()
    range.selectNodeContents(el)
    range.collapse(false)
    const selection = getSelection()
    selection?.removeAllRanges()
    selection?.addRange(range)
  }
  const rect = el.getBoundingClientRect()
  return { found: true, tag: el.tagName, role: el.getAttribute("role"), text: textOf(el).slice(0, 200), rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height } }
}

async function connectPage(endpoint: string | undefined, query?: string): Promise<BrowserCdpPage> {
  const target = await findTarget(endpoint, query)
  const connection = new BrowserCdpConnection(target.webSocketDebuggerUrl!)
  await connection.open()
  return new BrowserCdpPage(connection, target)
}

async function connectWebSocket(webSocketUrl: string, target: Partial<CdpTarget> = {}): Promise<BrowserCdpPage> {
  const connection = new BrowserCdpConnection(webSocketUrl)
  await connection.open()
  return new BrowserCdpPage(connection, {
    id: target.id || webSocketUrl,
    type: target.type || "page",
    title: target.title,
    url: target.url,
    webSocketDebuggerUrl: webSocketUrl,
  })
}

type ChatSendAssessment = {
  ok: boolean
  sent: boolean
  message: string
  response: string
  error?: string
  before: ChatPageState
  after: ChatPageState
  composer: ChatInputSelection & {
    beforeText: string
    afterText: string
    postText: string
  }
  responseMessages: ChatMessage[]
}

type ChatSendError = ChatSendAssessment

function normalizeText(text: string): string {
  return (text || "").replace(/\s+/g, " ").trim().toLowerCase()
}

function classifyDispatch(before: ChatPageState, after: ChatPageState, message: string): { sent: boolean; responseMessages: ChatMessage[] } {
  const normalized = normalizeText(message)
  const newMessages = after.messages.slice(before.totalCount)
  const responseMessages = newMessages.filter((entry) => entry.role === "assistant")
  const sentByCount = after.totalCount > before.totalCount
  const sentByMatch = newMessages.some((entry) => entry.role === "user" && normalizeText(entry.text).includes(normalized))
  return { sent: sentByCount || sentByMatch, responseMessages }
}

function pickResponse(after: ChatPageState): string {
  return after.messages.slice().reverse().find((entry) => entry.role === "assistant")?.text || ""
}

function makeSendError(message: string, reason: string, fallback: Partial<Pick<ChatSendAssessment, "before" | "after" | "composer" | "responseMessages" | "response">> = {}): ChatSendError {
  return {
    ok: false,
    sent: false,
    message,
    error: reason,
    response: fallback.response || "",
    before: fallback.before || { messages: [], totalCount: 0, userCount: 0, assistantCount: 0 },
    after: fallback.after || { messages: [], totalCount: 0, userCount: 0, assistantCount: 0 },
    composer: {
      found: false,
      selector: null,
      selectorHint: "",
      role: null,
      isContentEditable: false,
      isInput: false,
      tagName: "",
      textLabel: "",
      score: 0,
      reason,
      beforeText: "",
      afterText: "",
      postText: "",
      ...fallback.composer,
    },
    responseMessages: fallback.responseMessages || [],
  }
}

function findChatComposerForSession(targetHint?: string): ChatInputSelection {
  const hint = (targetHint || "").toLowerCase()
  const candidates = Array.from(document.querySelectorAll("textarea, input, [role='textbox'], [contenteditable='true'], [contenteditable='']"))
  const locationHint = document.location.href.toLowerCase()

  const ranked = candidates
    .map((element) => {
      const rect = element.getBoundingClientRect()
      const label = [
        element.getAttribute("aria-label"),
        element.getAttribute("placeholder"),
        element.getAttribute("data-placeholder"),
        element.getAttribute("data-testid"),
        element.getAttribute("data-test-id"),
      ].filter(Boolean).join(" ").toLowerCase()

      const style = getComputedStyle(element)
      if (style.display === "none" || style.visibility === "hidden") return null
      if (rect.width < 2 || rect.height < 2) return null
      if (
        element.tagName.toLowerCase() === "input" &&
        (element.getAttribute("type") || "").toLowerCase() !== "text" &&
        (element.getAttribute("type") || "").toLowerCase() !== "search"
      ) {
        return null
      }

      let score = defaultChatInputScore(element)
      if (hint && label.includes(hint)) score += 90
      if (hint && locationHint.includes(hint)) score += 80
      if (hint && String(element.textContent || "").toLowerCase().includes(hint)) score += 40
      const text = (element.textContent || "").toLowerCase()
      if (text.includes("chatgpt") || text.includes("ask") || text.includes("message") || text.includes("prompt")) score += 20

      return {
        element,
        score,
        label,
        rect,
      }
    })
    .filter((entry): entry is NonNullable<typeof entry> => Boolean(entry))

  if (!ranked.length) {
    return {
      found: false,
      selector: null,
      selectorHint: "no-match",
      role: null,
      isContentEditable: false,
      isInput: false,
      tagName: "",
      textLabel: "",
      score: 0,
      reason: "No visible composer candidate found in page.",
    }
  }

  const best = ranked
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score
      if (right.rect.bottom !== left.rect.bottom) return right.rect.bottom - left.rect.bottom
      return 0
    })[0]

  if (best.score < 35) {
    return {
      found: false,
      selector: null,
      selectorHint: best.element.tagName.toLowerCase(),
      role: best.element.getAttribute("role"),
      isContentEditable: (best.element as HTMLElement).isContentEditable || false,
      isInput: best.element instanceof HTMLInputElement || best.element instanceof HTMLTextAreaElement,
      tagName: best.element.tagName.toLowerCase(),
      textLabel: ([
        best.element.getAttribute("aria-label"),
        best.element.getAttribute("placeholder"),
        best.element.getAttribute("data-placeholder"),
        best.element.getAttribute("data-testid"),
        best.element.getAttribute("data-test-id"),
      ].filter(Boolean).join(" ").slice(0, 140)),
      score: best.score,
      reason: `Selection confidence too low (${best.score}); refusing to use it as composer.`,
    }
  }

  return {
    found: true,
    selector: buildCssSelector(best.element),
    selectorHint: best.element.tagName.toLowerCase(),
    role: best.element.getAttribute("role"),
    isContentEditable: (best.element as HTMLElement).isContentEditable || false,
    isInput: best.element instanceof HTMLInputElement || best.element instanceof HTMLTextAreaElement,
    tagName: best.element.tagName.toLowerCase(),
    textLabel: best.label.slice(0, 140),
    score: best.score,
    reason: `selected ${best.element.tagName.toLowerCase()} composer (${best.element.tagName.toLowerCase()}), score=${best.score}`,
  }
}

async function sendToChatGpt(message: string, options: { endpoint?: string; target?: string; webSocketUrl?: string; waitMs?: number } = {}) {
  const requestedMessage = String(message || "").trim()
  if (!requestedMessage) {
    return makeSendError(message, "Cannot send an empty message")
  }

  const target = (options.target || "chatgpt.com").trim()
  const expectedMessage = requestedMessage
  const expectedMessageNormalized = normalizeText(requestedMessage)
  const waitMs = typeof options.waitMs === "number" && options.waitMs >= 0 ? options.waitMs : 6000

  let before: ChatPageState = { messages: [], totalCount: 0, userCount: 0, assistantCount: 0 }
  let after: ChatPageState = { messages: [], totalCount: 0, userCount: 0, assistantCount: 0 }
  let selection: ChatInputSelection = {
    found: false,
    selector: null,
    selectorHint: "",
    role: null,
    isContentEditable: false,
    isInput: false,
    tagName: "",
    textLabel: "",
    score: 0,
    reason: "Selection did not run.",
  }
  let postComposer = { afterText: "" } as ChatInputPatch

  let page: BrowserCdpPage | undefined
  try {
    page = options.webSocketUrl
      ? await connectWebSocket(options.webSocketUrl, { url: target || "chatgpt.com" })
      : await connectPage(options.endpoint, target || "chatgpt.com")

    before = await page.evaluate<ChatPageState>(`(${readChatMessages.toString()})()`)
    selection = await page.evaluate<ChatInputSelection>(`(${findChatComposer.toString()})(${JSON.stringify(target)})`)
    if (!selection.found || !selection.selector) throw new Error(selection.reason || "No chat composer found")
    if (!selection.isContentEditable && !selection.isInput) throw new Error("Selected composer is not editable")

    const preComposer = await page.evaluate<ChatInputPatch>(`(${readChatComposer.toString()})(${JSON.stringify(selection.selector)})`)
    if (!preComposer.ok) throw new Error(preComposer.error || "Selected composer read failed")
    if (preComposer.isDisabled) throw new Error("Selected composer is disabled")

    const writeResult = await page.evaluate<ChatInputPatch>(`(${setChatComposer.toString()})(${JSON.stringify(selection.selector)}, ${JSON.stringify(expectedMessage)})`)
    if (!writeResult.ok) throw new Error(writeResult.error || "Failed to write to composer")

    const afterWrite = await page.evaluate<ChatInputPatch>(`(${readChatComposer.toString()})(${JSON.stringify(selection.selector)})`)
    if (normalizeText(writeResult.afterText || "") !== expectedMessageNormalized || normalizeText(afterWrite.afterText || "") !== expectedMessageNormalized) {
      throw new Error("Composer text verification failed")
    }

    await page.press("Enter")
    await new Promise((resolve) => setTimeout(resolve, waitMs))

    after = await page.evaluate<ChatPageState>(`(${readChatMessages.toString()})()`)
    postComposer = await page.evaluate<ChatInputPatch>(`(${readChatComposer.toString()})(${JSON.stringify(selection.selector)})`)
    const classify = classifyDispatch(before, after, expectedMessage)
    const postTextNormalized = normalizeText(postComposer.afterText || "")
    const response = pickResponse(after)
    const sent = classify.sent || response !== "" || postTextNormalized === ""
    if (!sent) {
      throw new Error("Send not confirmed: no new message appeared and composer still contains the submitted text")
    }

    return {
      ok: sent,
      sent,
      message: expectedMessage,
      response,
      before,
      after,
      composer: {
        ...selection,
        beforeText: preComposer.beforeText || "",
        afterText: writeResult.afterText || "",
        postText: postComposer.afterText || "",
      },
      responseMessages: classify.responseMessages,
    }
  } catch (error) {
    return makeSendError(expectedMessage, error instanceof Error ? error.message : String(error), {
      before,
      after,
      composer: {
        ...selection,
        beforeText: "",
        afterText: "",
        postText: postComposer.afterText || "",
      },
    })
  } finally {
    page?.close()
  }
}

export function bootstrapBrowserCdpRelay(): BrowserCdpRelay | undefined {
  if (typeof window === "undefined") return
  const existing: BrowserCdpRelay | undefined = window.edgerunCdpRelay
  if (existing) return existing

  let backendBridge: BackendBridge | null = null
  let chatSession = storedChatSession()
  const statusSubscribers = new Set<(state: BrowserCdpRelayState) => void>()
  const channel = "BroadcastChannel" in window ? new BroadcastChannel(FRONTEND_CHANNEL) : null
  channel?.addEventListener("message", (event) => {
    window.dispatchEvent(new CustomEvent(FRONTEND_EVENT, { detail: event.data }))
  })

  const getState = (): BrowserCdpRelayState => ({
    destination: relay.destination,
    frontend: "ready",
    backend: backendBridge ? "ready" : "offline",
    backendBridgeRegistered: Boolean(backendBridge),
    endpoint: relay.endpoint,
    chatSession,
    updatedAtIso: new Date().toISOString(),
  })

  const publishStatus = () => {
    const state = getState()
    for (const handler of statusSubscribers) handler(state)
    window.dispatchEvent(new CustomEvent("edgerun:cdp-relay-status", { detail: state }))
  }

  const relay: BrowserCdpRelay = {
    endpoint: DEFAULT_ENDPOINT,
    eventName: FRONTEND_EVENT,
    destination: storedDestination(),
    setChatSession(session: string) {
      const next = parseChatSession(session)
      chatSession = next
      try {
        window.localStorage.setItem(CHAT_SESSION_STORAGE_KEY, `${next.label}|${next.query}`)
      } catch {
        // Ignore storage failures; the in-memory session still updates.
      }
      publishStatus()
      return next.query
    },
    setEndpoint(endpoint: string) {
      relay.endpoint = normalizeEndpoint(endpoint)
      publishStatus()
      return relay.endpoint
    },
    setDestination(destination: RelayDestination) {
      relay.destination = destination
      try {
        window.localStorage.setItem(DESTINATION_STORAGE_KEY, destination)
      } catch {
        // Ignore storage failures; the in-memory route still updates.
      }
      publishStatus()
      return relay.destination
    },
    setBackendBridge(bridge: BackendBridge) {
      backendBridge = bridge
      publishStatus()
    },
    status() {
      return getState()
    },
    subscribeStatus(handler: (state: BrowserCdpRelayState) => void) {
      statusSubscribers.add(handler)
      handler(getState())
      return () => {
        statusSubscribers.delete(handler)
      }
    },
    targets() {
      return fetchTargets(relay.endpoint)
    },
    connect(query?: string) {
      return connectPage(relay.endpoint, query)
    },
    connectWebSocket(webSocketUrl: string, target?: Partial<CdpTarget>) {
      return connectWebSocket(webSocketUrl, target)
    },
    sendToChatGpt(message: string, options?: { target?: string; webSocketUrl?: string; waitMs?: number }) {
      return sendToChatGpt(message, { endpoint: relay.endpoint, target: options?.target || chatSession.query, ...options })
    },
    sendToFrontend(message: string, detail: Record<string, unknown> = {}) {
      const payload = { message, createdAtIso: new Date().toISOString(), ...detail }
      window.dispatchEvent(new CustomEvent(FRONTEND_EVENT, { detail: payload }))
      channel?.postMessage(payload)
      return payload
    },
    subscribe(handler: (message: Record<string, unknown>) => void) {
      const listener = (event: Event) => handler((event as CustomEvent<Record<string, unknown>>).detail)
      window.addEventListener(FRONTEND_EVENT, listener)
      return () => window.removeEventListener(FRONTEND_EVENT, listener)
    },
    async relay(message: string, destination: RelayDestination = relay.destination, options: Record<string, unknown> = {}) {
      if (destination === "frontend") return relay.sendToFrontend(message, options)
      if (destination === "backend") {
        if (!backendBridge) throw new Error("No backend bridge registered. Call window.edgerunCdpRelay.setBackendBridge(fn).")
        return await backendBridge(message, options)
      }
      if (destination === "chatgpt") {
        return await relay.sendToChatGpt(message, options as { target?: string; webSocketUrl?: string; waitMs?: number })
      }

      return sendToChatGpt(message, { endpoint: relay.endpoint, ...options as { target?: string; webSocketUrl?: string; waitMs?: number } })
    },
  }

  window.edgerunCdpRelay = relay
  return relay
}

declare global {
  interface Window {
    edgerunCdpRelay?: BrowserCdpRelay
  }
}
