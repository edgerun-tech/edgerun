import { writeFile } from "node:fs/promises"
import { CdpError, activateTarget, connectToTarget } from "./client.mjs"

const FIND_ELEMENT_SOURCE = String.raw`
function findElement(locator, root = document) {
  const options = typeof locator === "string" ? { selector: locator } : (locator || {});
  const visibleOnly = options.visible !== false;
  const all = (selector) => Array.from(root.querySelectorAll(selector));
  const textOf = (el) => [
    el.getAttribute?.("aria-label"),
    el.getAttribute?.("placeholder"),
    el.getAttribute?.("data-placeholder"),
    el.getAttribute?.("title"),
    el.innerText,
    el.textContent,
    el.value
  ].filter(Boolean).join(" ").replace(/\s+/g, " ").trim();
  const isVisible = (el) => {
    if (!visibleOnly) return true;
    const style = getComputedStyle(el);
    const rect = el.getBoundingClientRect();
    return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
  };
  const matchText = (el, needle, exact = false) => {
    const haystack = textOf(el);
    return exact ? haystack === needle : haystack.toLowerCase().includes(String(needle).toLowerCase());
  };
  let candidates = [];
  if (options.selector) candidates = all(options.selector);
  else if (options.role) candidates = all('[role="' + CSS.escape(options.role) + '"]');
  else if (options.placeholder) candidates = all("input,textarea,[contenteditable=true],[role=textbox]")
    .filter((el) => matchText(el, options.placeholder, options.exact));
  else if (options.text) candidates = all("button,a,input,textarea,[role=button],[role=link],[role=textbox],[contenteditable=true],label,[data-testid],h1,h2,h3,h4,h5,h6,p,span,div")
    .filter((el) => matchText(el, options.text, options.exact));
  else if (options.testId) candidates = all('[data-testid="' + CSS.escape(options.testId) + '"],[data-test-id="' + CSS.escape(options.testId) + '"]');
  candidates = candidates.filter(isVisible);
  const element = candidates[options.index || 0] || null;
  if (!element) return { found: false, candidates: [] };
  const rect = element.getBoundingClientRect();
  return {
    found: true,
    tag: element.tagName,
    role: element.getAttribute("role"),
    text: textOf(element).slice(0, 500),
    rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
    selector: options.selector || null
  };
}
`

export function locatorFromArg(arg) {
  if (!arg) return {}
  if (typeof arg !== "string") return arg
  if (arg.startsWith("text=")) return { text: arg.slice(5) }
  if (arg.startsWith("placeholder=")) return { placeholder: arg.slice(12) }
  if (arg.startsWith("role=")) return { role: arg.slice(5) }
  if (arg.startsWith("testid=")) return { testId: arg.slice(7) }
  return { selector: arg }
}

export class CdpPage {
  constructor(connection, target, options = {}) {
    this.connection = connection
    this.target = target
    this.endpoint = options.endpoint
  }

  static async connect(options = {}) {
    const { endpoint, target: query, timeoutMs } = options
    const { target, connection } = await connectToTarget(endpoint, query, { timeoutMs })
    return new CdpPage(connection, target, { endpoint })
  }

  async bringToFront() {
    if (this.endpoint && this.target?.id) {
      await activateTarget(this.endpoint, this.target.id).catch(() => undefined)
    }
    await this.connection.send("Page.bringToFront")
  }

  async enable() {
    await Promise.allSettled([
      this.connection.send("Page.enable"),
      this.connection.send("Runtime.enable"),
      this.connection.send("DOM.enable"),
      this.connection.send("Network.enable"),
    ])
  }

  async evaluate(expression, options = {}) {
    const result = await this.connection.send("Runtime.evaluate", {
      expression,
      awaitPromise: options.awaitPromise ?? true,
      returnByValue: options.returnByValue ?? true,
      userGesture: options.userGesture ?? true,
      includeCommandLineAPI: options.includeCommandLineAPI ?? true,
    }, options)
    if (result.exceptionDetails) {
      throw new CdpError("Runtime.evaluate threw", result.exceptionDetails)
    }
    return result.result?.value ?? result.result
  }

  async call(fn, args = [], options = {}) {
    const source = `(${fn.toString()})(...${JSON.stringify(args)})`
    return this.evaluate(source, options)
  }

  async find(locator) {
    return this.call(new Function("locator", `${FIND_ELEMENT_SOURCE}; return findElement(locator);`), [locatorFromArg(locator)])
  }

  async focus(locator) {
    return this.call(new Function("locator", `${FIND_ELEMENT_SOURCE};
      const match = findElement(locator);
      if (!match.found) return match;
      const options = typeof locator === "string" ? { selector: locator } : (locator || {});
      let el = null;
      if (options.selector) el = document.querySelector(options.selector);
      else {
        const matches = Array.from(document.querySelectorAll("textarea,input,[contenteditable=true],[role=textbox],button,a,[role=button]"));
        el = matches.find((candidate) => findElement({ ...options, selector: candidate.tagName ? undefined : options.selector }).found) || document.elementFromPoint(match.rect.x + match.rect.width / 2, match.rect.y + match.rect.height / 2);
      }
      el = el || document.elementFromPoint(match.rect.x + match.rect.width / 2, match.rect.y + match.rect.height / 2);
      el.scrollIntoView({ block: "center", inline: "center" });
      el.focus({ preventScroll: true });
      if (el.isContentEditable) {
        const range = document.createRange();
        range.selectNodeContents(el);
        range.collapse(false);
        const selection = getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
      }
      return { ...match, active: document.activeElement === el || el.contains(document.activeElement) };
    `), [locatorFromArg(locator)])
  }

  async click(locator) {
    const match = await this.find(locator)
    if (!match.found) return match
    await this.connection.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: match.rect.x + match.rect.width / 2,
      y: match.rect.y + match.rect.height / 2,
      button: "none",
    })
    await this.connection.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: match.rect.x + match.rect.width / 2,
      y: match.rect.y + match.rect.height / 2,
      button: "left",
      buttons: 1,
      clickCount: 1,
    })
    await this.connection.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: match.rect.x + match.rect.width / 2,
      y: match.rect.y + match.rect.height / 2,
      button: "left",
      buttons: 0,
      clickCount: 1,
    })
    return { ...match, clicked: true }
  }

  async type(text, options = {}) {
    if (options.locator) await this.focus(options.locator)
    await this.connection.send("Input.insertText", { text })
    return { typed: text.length }
  }

  async press(key, options = {}) {
    const code = key.length === 1 ? `Key${key.toUpperCase()}` : key
    const params = {
      key,
      code,
      windowsVirtualKeyCode: key.length === 1 ? key.toUpperCase().charCodeAt(0) : 0,
      nativeVirtualKeyCode: key.length === 1 ? key.toUpperCase().charCodeAt(0) : 0,
      modifiers: options.modifiers ?? 0,
    }
    await this.connection.send("Input.dispatchKeyEvent", { ...params, type: "keyDown" })
    await this.connection.send("Input.dispatchKeyEvent", { ...params, type: "keyUp" })
    return { pressed: key }
  }

  async navigate(url) {
    return this.connection.send("Page.navigate", { url })
  }

  async reload(options = {}) {
    return this.connection.send("Page.reload", { ignoreCache: options.ignoreCache ?? false })
  }

  async screenshot(path, options = {}) {
    const result = await this.connection.send("Page.captureScreenshot", {
      format: options.format ?? "png",
      fromSurface: options.fromSurface ?? true,
      captureBeyondViewport: options.captureBeyondViewport ?? true,
      quality: options.quality,
    })
    if (path) await writeFile(path, result.data, "base64")
    return { path, bytes: Math.floor(result.data.length * 0.75), data: path ? undefined : result.data }
  }

  async html(selector = "html") {
    return this.call((selectorArg) => document.querySelector(selectorArg)?.outerHTML ?? null, [selector])
  }

  async text(selector = "body") {
    return this.call((selectorArg) => document.querySelector(selectorArg)?.innerText ?? null, [selector])
  }

  async metrics() {
    const [metrics, performance] = await Promise.all([
      this.connection.send("Performance.getMetrics").catch(() => null),
      this.evaluate("JSON.parse(JSON.stringify(performance.getEntriesByType('navigation')[0] || {}))").catch(() => null),
    ])
    return { metrics: metrics?.metrics ?? [], navigation: performance }
  }

  async waitFor(locator, options = {}) {
    const timeoutMs = options.timeoutMs ?? 10_000
    const intervalMs = options.intervalMs ?? 100
    const started = Date.now()
    while (Date.now() - started < timeoutMs) {
      const match = await this.find(locator)
      if (match.found) return match
      await new Promise((resolve) => setTimeout(resolve, intervalMs))
    }
    throw new CdpError("Timed out waiting for locator", { locator, timeoutMs })
  }

  close() {
    this.connection.close()
  }
}
