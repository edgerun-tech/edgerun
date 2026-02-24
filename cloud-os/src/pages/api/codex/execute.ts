import type { APIRoute } from 'astro';
import { spawn } from 'node:child_process';
import path from 'node:path';

export const prerender = false;

const DEFAULT_TIMEOUT_MS = 120000;
const MAX_CAPTURE_BYTES = 512 * 1024;

type RequestBody = {
  prompt?: string;
  cwd?: string;
  timeoutMs?: number;
};

type CliResult = {
  ok: boolean;
  exitCode: number;
  durationMs: number;
  cwd: string;
  prompt: string;
  stdout: string;
  stderr: string;
  finalText: string;
};

export const POST: APIRoute = async ({ request }) => {
  try {
    const body = (await request.json()) as RequestBody;
    const prompt = (body.prompt || '').trim();
    if (!prompt) return json({ error: 'prompt is required' }, 400);
    if (prompt.length > 8000) return json({ error: 'prompt is too long (max 8000 chars)' }, 400);

    const workspaceRoot = process.env.CODEX_CLI_ROOT || process.cwd();
    const requestedCwd = body.cwd?.trim();
    const resolvedCwd = resolveCwd(workspaceRoot, requestedCwd);
    if (!resolvedCwd) return json({ error: 'cwd is outside allowed workspace root' }, 400);

    const timeoutMs = normalizeTimeout(body.timeoutMs);
    const result = await runCodexExec(prompt, resolvedCwd, timeoutMs);
    return json(result, result.ok ? 200 : 500);
  } catch (error: any) {
    return json({ error: error?.message || 'codex execute failed' }, 500);
  }
};

function runCodexExec(prompt: string, cwd: string, timeoutMs: number): Promise<CliResult> {
  return new Promise((resolve) => {
    const args = ['-a', 'never', '-s', 'workspace-write', 'exec', '--json', prompt];
    const child = spawn('codex', args, {
      cwd,
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdoutBuf = '';
    let stderrBuf = '';
    const startedAt = Date.now();
    let killedForTimeout = false;

    const timer = setTimeout(() => {
      killedForTimeout = true;
      child.kill('SIGTERM');
      setTimeout(() => child.kill('SIGKILL'), 1500).unref();
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      stdoutBuf = clampAppend(stdoutBuf, String(chunk), MAX_CAPTURE_BYTES);
    });
    child.stderr.on('data', (chunk) => {
      stderrBuf = clampAppend(stderrBuf, String(chunk), MAX_CAPTURE_BYTES);
    });
    child.on('error', (error) => {
      clearTimeout(timer);
      const durationMs = Date.now() - startedAt;
      resolve({
        ok: false,
        exitCode: -1,
        durationMs,
        cwd,
        prompt,
        stdout: stdoutBuf,
        stderr: `${stderrBuf}\n${error.message}`.trim(),
        finalText: '',
      });
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      const durationMs = Date.now() - startedAt;
      const finalText = parseCodexFinalText(stdoutBuf);
      const timeoutNote = killedForTimeout ? '\nTimed out waiting for codex response.' : '';
      resolve({
        ok: code === 0 && !killedForTimeout,
        exitCode: code ?? -1,
        durationMs,
        cwd,
        prompt,
        stdout: stdoutBuf.trim(),
        stderr: `${stderrBuf}${timeoutNote}`.trim(),
        finalText,
      });
    });
  });
}

function parseCodexFinalText(stdout: string): string {
  const lines = stdout
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    try {
      const parsed = JSON.parse(line) as { type?: string; item?: { text?: string } };
      if (parsed.type === 'item.completed' && parsed.item?.text) return parsed.item.text;
    } catch {
      continue;
    }
  }
  return '';
}

function clampAppend(current: string, next: string, limitBytes: number): string {
  const combined = `${current}${next}`;
  if (Buffer.byteLength(combined, 'utf8') <= limitBytes) return combined;
  const overflow = Buffer.byteLength(combined, 'utf8') - limitBytes;
  return combined.slice(Math.max(0, overflow));
}

function resolveCwd(workspaceRoot: string, requestedCwd?: string): string | null {
  const root = path.resolve(workspaceRoot);
  if (!requestedCwd) return root;
  const resolved = path.resolve(requestedCwd);
  if (resolved === root || resolved.startsWith(`${root}${path.sep}`)) return resolved;
  return null;
}

function normalizeTimeout(value?: number): number {
  if (!value || Number.isNaN(value)) return DEFAULT_TIMEOUT_MS;
  return Math.min(Math.max(Math.floor(value), 5000), 300000);
}

function json(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}
