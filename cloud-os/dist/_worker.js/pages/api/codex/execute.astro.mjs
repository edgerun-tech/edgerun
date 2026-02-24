globalThis.process ??= {}; globalThis.process.env ??= {};
import { spawn } from 'node:child_process';
import path from 'node:path';
export { r as renderers } from '../../../chunks/_@astro-renderers_B30lzduo.mjs';

const prerender = false;
const DEFAULT_TIMEOUT_MS = 12e4;
const MAX_CAPTURE_BYTES = 512 * 1024;
const POST = async ({ request }) => {
  try {
    const body = await request.json();
    const prompt = (body.prompt || "").trim();
    if (!prompt) return json({ error: "prompt is required" }, 400);
    if (prompt.length > 8e3) return json({ error: "prompt is too long (max 8000 chars)" }, 400);
    const workspaceRoot = process.env.CODEX_CLI_ROOT || process.cwd();
    const requestedCwd = body.cwd?.trim();
    const resolvedCwd = resolveCwd(workspaceRoot, requestedCwd);
    if (!resolvedCwd) return json({ error: "cwd is outside allowed workspace root" }, 400);
    const timeoutMs = normalizeTimeout(body.timeoutMs);
    const result = await runCodexExec(prompt, resolvedCwd, timeoutMs);
    return json(result, result.ok ? 200 : 500);
  } catch (error) {
    return json({ error: error?.message || "codex execute failed" }, 500);
  }
};
function runCodexExec(prompt, cwd, timeoutMs) {
  return new Promise((resolve) => {
    const args = ["-a", "never", "-s", "workspace-write", "exec", "--json", prompt];
    const child = spawn("codex", args, {
      cwd,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"]
    });
    let stdoutBuf = "";
    let stderrBuf = "";
    const startedAt = Date.now();
    let killedForTimeout = false;
    const timer = setTimeout(() => {
      killedForTimeout = true;
      child.kill("SIGTERM");
      setTimeout(() => child.kill("SIGKILL"), 1500).unref();
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdoutBuf = clampAppend(stdoutBuf, String(chunk), MAX_CAPTURE_BYTES);
    });
    child.stderr.on("data", (chunk) => {
      stderrBuf = clampAppend(stderrBuf, String(chunk), MAX_CAPTURE_BYTES);
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      const durationMs = Date.now() - startedAt;
      resolve({
        ok: false,
        exitCode: -1,
        durationMs,
        cwd,
        prompt,
        stdout: stdoutBuf,
        stderr: `${stderrBuf}
${error.message}`.trim(),
        finalText: ""
      });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      const durationMs = Date.now() - startedAt;
      const finalText = parseCodexFinalText(stdoutBuf);
      const timeoutNote = killedForTimeout ? "\nTimed out waiting for codex response." : "";
      resolve({
        ok: code === 0 && !killedForTimeout,
        exitCode: code ?? -1,
        durationMs,
        cwd,
        prompt,
        stdout: stdoutBuf.trim(),
        stderr: `${stderrBuf}${timeoutNote}`.trim(),
        finalText
      });
    });
  });
}
function parseCodexFinalText(stdout) {
  const lines = stdout.split("\n").map((line) => line.trim()).filter((line) => line.length > 0);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    try {
      const parsed = JSON.parse(line);
      if (parsed.type === "item.completed" && parsed.item?.text) return parsed.item.text;
    } catch {
      continue;
    }
  }
  return "";
}
function clampAppend(current, next, limitBytes) {
  const combined = `${current}${next}`;
  if (Buffer.byteLength(combined, "utf8") <= limitBytes) return combined;
  const overflow = Buffer.byteLength(combined, "utf8") - limitBytes;
  return combined.slice(Math.max(0, overflow));
}
function resolveCwd(workspaceRoot, requestedCwd) {
  const root = path.resolve(workspaceRoot);
  if (!requestedCwd) return root;
  const resolved = path.resolve(requestedCwd);
  if (resolved === root || resolved.startsWith(`${root}${path.sep}`)) return resolved;
  return null;
}
function normalizeTimeout(value) {
  if (!value || Number.isNaN(value)) return DEFAULT_TIMEOUT_MS;
  return Math.min(Math.max(Math.floor(value), 5e3), 3e5);
}
function json(payload, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "Content-Type": "application/json" }
  });
}

const _page = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  POST,
  prerender
}, Symbol.toStringTag, { value: 'Module' }));

const page = () => _page;

export { page };
