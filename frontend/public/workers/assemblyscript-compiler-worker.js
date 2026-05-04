function base64ToBytes(base64) {
  const bin = atob(base64)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return bytes
}

function stringifyExportResult(value) {
  if (typeof value === "bigint") return value.toString()
  if (value === undefined) return "undefined"
  return String(value)
}

async function instantiateAndRun(wasm) {
  const started = performance.now()
  const imports = {
    env: {
      abort(message, fileName, line, column) {
        throw new Error(`abort at ${line}:${column} message=${message} file=${fileName}`)
      },
    },
  }

  const { instance } = await WebAssembly.instantiate(wasm, imports)
  const exports = instance.exports
  const exportNames = Object.keys(exports)
  const callable = exports.run
  const runResult = typeof callable === "function" ? stringifyExportResult(callable()) : undefined

  return {
    instantiateMs: performance.now() - started,
    exports: exportNames,
    runResult,
  }
}

self.onmessage = async (event) => {
  const { id, source } = event.data
  const logs = []

  try {
    logs.push("sending source to AssemblyScript compile API...")

    const response = await fetch("/api/assemblyscript/compile", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source }),
    })

    const compiled = await response.json()
    logs.push(...(compiled.logs || []))

    if (!compiled.ok) {
      throw new Error(compiled.error || `compile failed with HTTP ${response.status}`)
    }

    const wasm = base64ToBytes(compiled.wasmBase64)
    const run = await instantiateAndRun(wasm)
    logs.push(`instantiated module in ${run.instantiateMs.toFixed(2)}ms`)
    logs.push(`exports: ${run.exports.join(", ") || "none"}`)
    logs.push(run.runResult === undefined ? "no exported run() function found; module compiled successfully" : `run() → ${run.runResult}`)

    self.postMessage({
      id,
      ok: true,
      logs,
      wasm,
      wasmSize: wasm.byteLength,
      compileMs: compiled.compileMs || 0,
      instantiateMs: run.instantiateMs,
      exports: run.exports,
      runResult: run.runResult,
    }, [wasm.buffer])
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      logs,
      error: error instanceof Error ? error.message : String(error),
      compileMs: 0,
      instantiateMs: 0,
    })
  }
}
