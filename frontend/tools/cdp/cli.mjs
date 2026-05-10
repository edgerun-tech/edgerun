#!/usr/bin/env node
import { getBrowserVersion, listTargets, newTarget, normalizeEndpoint } from "./client.mjs"
import { CdpPage, locatorFromArg } from "./page.mjs"

function parseArgs(argv) {
  const args = { _: [] }
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index]
    if (!value.startsWith("--")) {
      args._.push(value)
      continue
    }
    const [key, inline] = value.slice(2).split("=", 2)
    if (inline !== undefined) args[key] = inline
    else if (argv[index + 1] && !argv[index + 1].startsWith("--")) args[key] = argv[++index]
    else args[key] = true
  }
  return args
}

function usage() {
  return `CDP toolkit

Usage:
  node tools/cdp/cli.mjs <command> [args] [--endpoint 127.0.0.1:9222] [--target query]

Commands:
  version                         Print browser CDP version metadata
  list                            List debuggable targets
  new <url>                       Open a new target
  eval <js>                       Evaluate JavaScript in the selected page
  find <locator>                  Find an element and print its metadata
  focus <locator>                 Focus an element
  click <locator>                 Click an element center
  type <text> [--locator loc]     Insert text, optionally focusing first
  press <key>                     Dispatch a key press
  navigate <url>                  Navigate selected page
  reload [--ignore-cache]         Reload selected page
  screenshot <file>               Capture PNG screenshot
  html [selector]                 Print outerHTML for selector
  text [selector]                 Print innerText for selector
  wait <locator> [--timeout ms]   Wait until an element is visible
  metrics                         Print performance metrics
  observe [--duration ms]         Collect console/log/network events
  cookies [url] [--raw]           Print cookies visible to the page
  storage [--raw]                 Print localStorage and sessionStorage

Locators:
  CSS selectors by default, or text=Save, placeholder=Ask anything, role=textbox, testid=submit.
`
}

function print(value, { raw = false } = {}) {
  if (raw && typeof value === "string") {
    process.stdout.write(value)
    if (!value.endsWith("\n")) process.stdout.write("\n")
    return
  }
  console.log(JSON.stringify(value, null, 2))
}

async function withPage(args, fn) {
  const page = await CdpPage.connect({
    endpoint: args.endpoint,
    target: args.target,
    timeoutMs: Number(args.timeout || args["command-timeout"] || 10_000),
  })
  try {
    if (args.front !== "false") await page.bringToFront()
    await page.enable()
    return await fn(page)
  } finally {
    page.close()
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const command = args._[0]
  const endpoint = normalizeEndpoint(args.endpoint)
  if (!command || command === "help" || command === "--help") {
    process.stdout.write(usage())
    return
  }

  if (command === "version") {
    print(await getBrowserVersion(endpoint))
    return
  }
  if (command === "list") {
    const targets = await listTargets(endpoint)
    print(targets.map((target) => ({
      id: target.id,
      type: target.type,
      title: target.title,
      url: target.url,
      websocket: target.webSocketDebuggerUrl,
    })))
    return
  }
  if (command === "new") {
    print(await newTarget(endpoint, args._[1] || "about:blank"))
    return
  }

  const result = await withPage(args, async (page) => {
    if (command === "eval") return page.evaluate(args._.slice(1).join(" "))
    if (command === "find") return page.find(locatorFromArg(args._[1]))
    if (command === "focus") return page.focus(locatorFromArg(args._[1]))
    if (command === "click") return page.click(locatorFromArg(args._[1]))
    if (command === "type") return page.type(args._.slice(1).join(" "), { locator: args.locator && locatorFromArg(args.locator) })
    if (command === "press") return page.press(args._[1] || "Enter")
    if (command === "navigate") return page.navigate(args._[1])
    if (command === "reload") return page.reload({ ignoreCache: Boolean(args["ignore-cache"]) })
    if (command === "screenshot") return page.screenshot(args._[1] || "cdp-screenshot.png")
    if (command === "html") return page.html(args._[1] || "html")
    if (command === "text") return page.text(args._[1] || "body")
    if (command === "wait") return page.waitFor(locatorFromArg(args._[1]), { timeoutMs: Number(args.timeout || 10_000) })
    if (command === "metrics") return page.metrics()
    if (command === "observe") return page.observe({ durationMs: Number(args.duration || 5_000) })
    if (command === "cookies") return page.cookies(args._[1] ? [args._[1]] : [], { raw: Boolean(args.raw) })
    if (command === "storage") return page.storage({ raw: Boolean(args.raw) })
    throw new Error(`Unknown command: ${command}\n\n${usage()}`)
  })
  print(result, { raw: command === "html" || command === "text" })
}

main().catch((error) => {
  console.error(error?.stack || error?.message || String(error))
  process.exitCode = 1
})
