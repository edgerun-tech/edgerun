/**
 * Bun dev server — builds & runs the Rust binary, serves the frontend.
 * The frontend connects directly to the Rust backend's WS on port 13337.
 *
 * Usage:
 *   bun run viewer/src/dev-server.js              # port 5173, analyze ./samples
 *   bun run viewer/src/dev-server.js --port 3000  # custom port
 *   bun run viewer/src/dev-server.js --analyze /path/to/code  # codebase to analyze
 */

const DEV_PORT = Number(process.env.DEV_PORT ?? 5173);

// ─── Parse CLI args ──────────────────────────────────────────────────

const args = process.argv.slice(2);
let port = DEV_PORT;
let analyzePath = null;

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--port" && args[i + 1]) {
    port = Number(args[++i]);
  } else if (args[i] === "--analyze" && args[i + 1]) {
    analyzePath = args[++i];
  } else if (args[i] === "--help" || args[i] === "-h") {
    console.log("Usage: bun run viewer/src/dev-server.js [--port <port>] [--analyze <path>]");
    process.exit(0);
  }
}

analyzePath ??= process.env.CODEANALYZE_PATH ?? "/home/ken/edgerun_reference_core/crates/edgerun-agent";

console.log(`🚀 Dev server on http://localhost:${port}`);
console.log(`   Backend: managed (port 13337)`);

// ─── Rust process management ─────────────────────────────────────────

let rustProc = null;
let rustReady = false;

async function buildRust() {
  console.log("\n🔨 Building Rust binary…");
  const build = Bun.spawn(["cargo", "build"], {
    cwd: import.meta.dir.split("/viewer/src")[0],
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await build.exited;
  if (code !== 0) {
    console.error("❌ Rust build failed");
    return false;
  }
  console.log("✅ Rust build complete");
  return true;
}

function startRust(pathToAnalyze) {
  if (rustProc) {
    console.log("🔄 Restarting Rust binary…");
    rustProc.kill();
    rustProc = null;
    rustReady = false;
  }

  console.log(`▶ Starting codeanalyzer (analyzing: ${pathToAnalyze})`);
  rustProc = Bun.spawn(
    ["cargo", "run", "--bin", "codeanalyzer", "--", pathToAnalyze, "--port", "13337"],
    {
      cwd: import.meta.dir.split("/viewer/src")[0],
      stdout: "pipe",
      stderr: "pipe",
      onExit(proc) {
        console.log(`⚠ Rust process exited (code ${proc.exitCode})`);
        rustProc = null;
        rustReady = false;
      },
    },
  );

  // Stream output
  void pumpStream(rustProc.stdout);
  void pumpStream(rustProc.stderr);

  // Wait for the server to be ready
  void waitForRust();
}

async function pumpStream(stream) {
  if (!stream) return;
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      process.stdout.write(decoder.decode(value, { stream: true }));
    }
  } catch {
    // stream closed
  }
}

async function waitForRust() {
  for (let i = 0; i < 60; i++) {
    await sleep(500);
    try {
      const resp = await fetch("http://localhost:13337/", { method: "HEAD" });
      if (resp.ok || resp.status === 200 || resp.status === 304) {
        rustReady = true;
        console.log("✅ Rust backend ready\n");
        return;
      }
    } catch {
      // not ready yet
    }
  }
  console.warn("⚠ Rust backend took too long to start");
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

// ─── Initial build & start ───────────────────────────────────────────

if (await buildRust()) {
  startRust(analyzePath);
} else {
  console.error("Cannot start dev server without a working Rust binary.");
  process.exit(1);
}

// ─── Watch Rust source files ─────────────────────────────────────────

{
  const rustSrcDir = import.meta.dir.split("/viewer/src")[0] + "/src";
  try {
    const fs = await import("node:fs");
    let debounce = null;

    function onChange() {
      if (!rustReady) return;
      if (debounce) clearTimeout(debounce);
      debounce = setTimeout(async () => {
        debounce = null;
        console.log("\n📝 Rust source changed…");
        const ok = await buildRust();
        if (ok) startRust(analyzePath);
      }, 1500);
    }

    fs.watch(rustSrcDir, { recursive: true }, onChange);
    console.log(`👀 Watching ${rustSrcDir} for changes…`);
  } catch (e) {
    console.warn("⚠ Could not watch Rust source files:", e.message);
  }
}

// ─── Static file server ──────────────────────────────────────────────

// Static-file MIME types only; not codelyzer wire protocol framing.
const MIME_MAP = {
  html: "text/html",
  css: "text/css",
  js: "application/javascript",
  mjs: "application/javascript",
  map: "application/json",
  json: "application/json",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  svg: "image/svg+xml",
  ico: "image/x-icon",
  woff2: "font/woff2",
  ttf: "font/ttf",
};

const server = Bun.serve({
  port,
  async fetch(req) {
    const url = new URL(req.url);

    // Root → index.html
    if (url.pathname === "/" || url.pathname === "/index.html") {
      const file = Bun.file("viewer/index.html");
      return new Response(file, {
        headers: { "Content-Type": "text/html; charset=utf-8" },
      });
    }

    // /dist/* → viewer/dist/*
    if (url.pathname.startsWith("/dist/")) {
      const filePath = `viewer${url.pathname}`;
      const file = Bun.file(filePath);
      if (await file.exists()) {
        const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
        const mime = MIME_MAP[ext] ?? "application/octet-stream";
        return new Response(file, {
          headers: {
            "Content-Type": `${mime}; charset=utf-8`,
            "Cache-Control": "no-cache",
          },
        });
      }
      return new Response("Not Found", { status: 404 });
    }

    return new Response("Not Found", { status: 404 });
  },
});

// ─── Cleanup on exit ─────────────────────────────────────────────────

for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => {
    console.log("\n👋 Shutting down…");
    rustProc?.kill();
    server.stop();
    process.exit(0);
  });
}
