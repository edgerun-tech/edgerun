// SPDX-License-Identifier: Apache-2.0

export function installLocalBridgeSimulator(win) {
  const CREDENTIALS_STORAGE_KEY = 'intent-ui-local-bridge-credentials-sim-v1'
  const MCP_RUNTIME_STORAGE_KEY = 'intent-ui-local-bridge-mcp-sim-v1'

  const readCredentials = () => {
    try {
      const parsed = JSON.parse(String(win.localStorage.getItem(CREDENTIALS_STORAGE_KEY) || '[]'))
      return Array.isArray(parsed) ? parsed : []
    } catch {
      return []
    }
  }

  const writeCredentials = (entries) => {
    win.localStorage.setItem(CREDENTIALS_STORAGE_KEY, JSON.stringify(Array.isArray(entries) ? entries : []))
  }

  const readMcpRuntime = () => {
    try {
      const parsed = JSON.parse(String(win.localStorage.getItem(MCP_RUNTIME_STORAGE_KEY) || '{}'))
      return parsed && typeof parsed === 'object' ? parsed : {}
    } catch {
      return {}
    }
  }

  const writeMcpRuntime = (state) => {
    const next = state && typeof state === 'object' ? state : {}
    win.localStorage.setItem(MCP_RUNTIME_STORAGE_KEY, JSON.stringify(next))
  }

  class FakeBridgeWebSocket {
    constructor(url) {
      this.url = url
      this.readyState = 0
      this.binaryType = 'arraybuffer'
      setTimeout(() => {
        this.readyState = 1
        if (typeof this.onopen === 'function') this.onopen()
      }, 0)
    }

    send(payload) {
      const data = payload instanceof Uint8Array
        ? payload.buffer.slice(payload.byteOffset, payload.byteOffset + payload.byteLength)
        : payload
      setTimeout(() => {
        if (typeof this.onmessage === 'function') this.onmessage({ data })
      }, 0)
    }

    close() {
      this.readyState = 3
      if (typeof this.onclose === 'function') this.onclose()
    }
  }

  const originalFetch = win.fetch.bind(win)
  win.fetch = (input, init) => {
    const url = String(typeof input === 'string' ? input : input?.url || '')
    const pathname = (() => {
      try {
        return new URL(url, win.location.origin).pathname
      } catch {
        return ''
      }
    })()
    if (url.includes('/v1/local/node/info.pb')) {
      return Promise.resolve(
        new win.Response(new Uint8Array([8, 1]), {
          status: 200,
          headers: { 'content-type': 'application/octet-stream' }
        })
      )
    }
    if (pathname === '/v1/local/credentials/status') {
      const entries = readCredentials()
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          installed: true,
          locked: false,
          count: entries.length,
          backend: 'tpm'
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/credentials/list') {
      const entries = readCredentials()
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          entries,
          count: entries.length,
          locked: false
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/credentials/store' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const entries = readCredentials()
      const now = Date.now()
      const entryId = String(body?.entryId || body?.name || '').trim()
      const name = String(body?.name || body?.entryId || '').trim()
      const existingIndex = entries.findIndex((entry) => String(entry?.entry_id || '').trim() === entryId || String(entry?.name || '').trim() === name)
      const next = {
        entry_id: entryId || name,
        credentialType: String(body?.credentialType || 'secret').trim(),
        credential_type: String(body?.credentialType || 'secret').trim(),
        name,
        username: String(body?.username || '').trim(),
        secret: String(body?.secret || '').trim(),
        url: String(body?.url || '').trim(),
        note: String(body?.note || '').trim(),
        tags: String(body?.tags || '').trim(),
        folder: String(body?.folder || '').trim(),
        created_unix_ms: now,
        updated_unix_ms: now
      }
      if (existingIndex >= 0) {
        next.created_unix_ms = Number(entries[existingIndex]?.created_unix_ms || now)
        entries[existingIndex] = next
      } else {
        entries.push(next)
      }
      writeCredentials(entries)
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/credentials/delete' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const entryId = String(body?.entryId || '').trim()
      const entries = readCredentials().filter((entry) => String(entry?.entry_id || '').trim() !== entryId)
      writeCredentials(entries)
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/credentials/unlock' || pathname === '/v1/local/credentials/lock') {
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, locked: false }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/credentials/integration-token') {
      const integrationId = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('integration_id') || '').trim()
        } catch {
          return ''
        }
      })()
      const tokenName = `integration/${integrationId}/token`
      const entry = readCredentials().find((candidate) => String(candidate?.name || '').trim() === tokenName)
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          integration_id: integrationId,
          token: String(entry?.secret || '')
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/mcp/integration/start' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const integrationId = String(body?.integration_id || '').trim().toLowerCase()
      const token = String(body?.token || '').trim()
      if (!integrationId) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'integration_id is required' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      if (token.length < 8) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'integration token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const runtimes = readMcpRuntime()
      runtimes[integrationId] = {
        integration_id: integrationId,
        container_name: `edgerun-mcp-${integrationId.replace(/[^a-z0-9-]/g, '-')}`,
        running: true,
        status: 'running'
      }
      writeMcpRuntime(runtimes)
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          error: '',
          data: runtimes[integrationId]
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/mcp/integration/stop' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const integrationId = String(body?.integration_id || '').trim().toLowerCase()
      if (!integrationId) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'integration_id is required' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const runtimes = readMcpRuntime()
      runtimes[integrationId] = {
        integration_id: integrationId,
        container_name: `edgerun-mcp-${integrationId.replace(/[^a-z0-9-]/g, '-')}`,
        running: false,
        status: 'stopped'
      }
      writeMcpRuntime(runtimes)
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          error: '',
          data: runtimes[integrationId]
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/mcp/integration/status') {
      const integrationId = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('integration_id') || '').trim().toLowerCase()
        } catch {
          return ''
        }
      })()
      if (!integrationId) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'integration_id is required' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const runtimes = readMcpRuntime()
      const runtime = runtimes[integrationId] || {
        integration_id: integrationId,
        container_name: `edgerun-mcp-${integrationId.replace(/[^a-z0-9-]/g, '-')}`,
        running: false,
        status: 'not_found'
      }
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          error: '',
          data: runtime
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    return originalFetch(input, init)
  }

  win.WebSocket = FakeBridgeWebSocket
}
