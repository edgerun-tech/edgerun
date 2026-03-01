// SPDX-License-Identifier: Apache-2.0

export function installLocalBridgeSimulator(win) {
  const CREDENTIALS_STORAGE_KEY = 'intent-ui-local-bridge-credentials-sim-v1'
  const MCP_RUNTIME_STORAGE_KEY = 'intent-ui-local-bridge-mcp-sim-v1'
  const CLOUDFLARE_STORAGE_KEY = 'intent-ui-local-bridge-cloudflare-sim-v1'
  const DOCKER_STORAGE_KEY = 'intent-ui-local-bridge-docker-sim-v1'
  const STRICT_NODE_CHECK_STORAGE_KEY = 'intent-ui-local-bridge-strict-node-sim-v1'
  const DISABLE_HOST_INFO_STORAGE_KEY = 'intent-ui-local-bridge-disable-host-info-sim-v1'

  const strictNodeCheckEnabled = () => {
    const raw = String(win.localStorage.getItem(STRICT_NODE_CHECK_STORAGE_KEY) || '').trim().toLowerCase()
    return raw === '1' || raw === 'true' || raw === 'yes'
  }

  const hostInfoDisabled = () => {
    const raw = String(win.localStorage.getItem(DISABLE_HOST_INFO_STORAGE_KEY) || '').trim().toLowerCase()
    return raw === '1' || raw === 'true' || raw === 'yes'
  }

  const rejectIfNodeIdProvided = (nodeId) => {
    if (!strictNodeCheckEnabled()) return null
    if (!String(nodeId || '').trim()) return null
    return new win.Response(JSON.stringify({
      ok: false,
      error: 'selected node is not local; local bridge filesystem is only available for node node-sim'
    }), {
      status: 403,
      headers: { 'content-type': 'application/json; charset=utf-8' }
    })
  }

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

  const defaultCloudflareState = () => ({
    accountId: 'acc-sim-1',
    zones: [
      { id: 'zone-sim-1', name: 'example.com', status: 'active' },
      { id: 'zone-sim-2', name: 'edge.run', status: 'active' }
    ],
    tunnels: [
      { id: 'tnl-sim-1', name: 'edge-terminal', status: 'healthy' }
    ],
    accessApps: [
      { id: 'app-sim-1', name: 'Terminal Access', domain: 'terminal.example.com' }
    ],
    workers: [
      { id: 'edge-worker', modified_on: '2026-03-01T00:00:00.000Z' }
    ],
    pages: [
      { id: 'pages-sim-1', name: 'edge-site', subdomain: 'edge-site.pages.dev' }
    ],
    dnsByZone: {
      'zone-sim-1': [
        { id: 'dns-sim-1', type: 'CNAME', name: 'terminal.example.com', content: 'edge-terminal.cfargotunnel.com' }
      ],
      'zone-sim-2': []
    }
  })

  const readCloudflareState = () => {
    try {
      const parsed = JSON.parse(String(win.localStorage.getItem(CLOUDFLARE_STORAGE_KEY) || '{}'))
      if (parsed && typeof parsed === 'object') {
        return {
          ...defaultCloudflareState(),
          ...parsed,
          zones: Array.isArray(parsed.zones) ? parsed.zones : defaultCloudflareState().zones,
          tunnels: Array.isArray(parsed.tunnels) ? parsed.tunnels : defaultCloudflareState().tunnels,
          accessApps: Array.isArray(parsed.accessApps) ? parsed.accessApps : defaultCloudflareState().accessApps,
          workers: Array.isArray(parsed.workers) ? parsed.workers : defaultCloudflareState().workers,
          pages: Array.isArray(parsed.pages) ? parsed.pages : defaultCloudflareState().pages,
          dnsByZone: parsed.dnsByZone && typeof parsed.dnsByZone === 'object' ? parsed.dnsByZone : defaultCloudflareState().dnsByZone
        }
      }
    } catch {
      // ignore parse errors
    }
    return defaultCloudflareState()
  }

  const writeCloudflareState = (state) => {
    win.localStorage.setItem(CLOUDFLARE_STORAGE_KEY, JSON.stringify(state && typeof state === 'object' ? state : defaultCloudflareState()))
  }

  const defaultDockerState = () => ({
    swarm_active: false,
    swarm_node_id: '',
    services: [],
    containers: [
      {
        id: 'ctr-sim-1',
        name: 'edgerun-dev-api',
        image: 'ghcr.io/edgerun/dev-api:latest',
        status: 'Up 2 hours',
        state: 'running',
        ports: '0.0.0.0:7001->7001/tcp'
      }
    ]
  })

  const readDockerState = () => {
    try {
      const parsed = JSON.parse(String(win.localStorage.getItem(DOCKER_STORAGE_KEY) || '{}'))
      if (parsed && typeof parsed === 'object') {
        const fallback = defaultDockerState()
        return {
          ...fallback,
          ...parsed,
          services: Array.isArray(parsed.services) ? parsed.services : fallback.services,
          containers: Array.isArray(parsed.containers) ? parsed.containers : fallback.containers
        }
      }
    } catch {
      // ignore parse errors
    }
    return defaultDockerState()
  }

  const writeDockerState = (state) => {
    win.localStorage.setItem(DOCKER_STORAGE_KEY, JSON.stringify(state && typeof state === 'object' ? state : defaultDockerState()))
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
      if (hostInfoDisabled()) {
        return Promise.resolve(
          new win.Response(new Uint8Array(), {
            status: 503,
            headers: { 'content-type': 'application/octet-stream' }
          })
        )
      }
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
    if (pathname === '/v1/local/docker/summary') {
      const state = readDockerState()
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          error: '',
          swarm_active: Boolean(state.swarm_active),
          swarm_node_id: String(state.swarm_node_id || ''),
          services: Array.isArray(state.services) ? state.services : [],
          containers: Array.isArray(state.containers) ? state.containers : []
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/docker/container/state' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const action = String(body?.action || '').trim().toLowerCase()
      const selector = String(body?.container || '').trim()
      if (!selector) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'container selector is required' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      if (!['start', 'stop', 'restart'].includes(action)) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'action must be start, stop, or restart' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readDockerState()
      const containers = Array.isArray(state.containers) ? [...state.containers] : []
      const index = containers.findIndex((entry) => {
        const id = String(entry?.id || '').trim()
        const name = String(entry?.name || '').trim()
        return selector === id || selector === name
      })
      if (index < 0) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: `container not found: ${selector}` }), {
            status: 404,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const next = { ...containers[index] }
      if (action === 'stop') {
        next.state = 'exited'
        next.status = 'Exited (0) 1 second ago'
      } else {
        next.state = 'running'
        next.status = action === 'restart' ? 'Up 1 second (restarted)' : 'Up 1 second'
      }
      containers[index] = next
      writeDockerState({
        ...state,
        containers
      })
      return Promise.resolve(
        new win.Response(JSON.stringify({
          ok: true,
          container: selector,
          action,
          state: next.state,
          message: 'container state updated'
        }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/zones') {
      const token = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('token') || '').trim()
        } catch {
          return ''
        }
      })()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, zones: state.zones, count: state.zones.length }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/tunnels') {
      const token = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('token') || '').trim()
        } catch {
          return ''
        }
      })()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, account_id: state.accountId, tunnels: state.tunnels, count: state.tunnels.length }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/access/apps') {
      const token = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('token') || '').trim()
        } catch {
          return ''
        }
      })()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, account_id: state.accountId, apps: state.accessApps, count: state.accessApps.length }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/workers') {
      const token = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('token') || '').trim()
        } catch {
          return ''
        }
      })()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, account_id: state.accountId, workers: state.workers, count: state.workers.length }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/pages') {
      const token = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('token') || '').trim()
        } catch {
          return ''
        }
      })()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, account_id: state.accountId, pages: state.pages, count: state.pages.length }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/dns/records') {
      const token = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('token') || '').trim()
        } catch {
          return ''
        }
      })()
      const zoneId = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('zone_id') || '').trim()
        } catch {
          return ''
        }
      })()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      if (!zoneId) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'zone_id is required' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      const records = Array.isArray(state.dnsByZone?.[zoneId]) ? state.dnsByZone[zoneId] : []
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, zone_id: zoneId, records, count: records.length }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/cloudflare/dns/records/upsert' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const token = String(body?.token || '').trim()
      const zoneId = String(body?.zone_id || '').trim()
      const name = String(body?.name || '').trim()
      const content = String(body?.content || '').trim()
      const type = String(body?.record_type || 'CNAME').trim().toUpperCase()
      if (token.length < 20) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'cloudflare account api token is missing or invalid' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      if (!zoneId || !name || !content) {
        return Promise.resolve(
          new win.Response(JSON.stringify({ ok: false, error: 'zone_id, name, and content are required' }), {
            status: 400,
            headers: { 'content-type': 'application/json; charset=utf-8' }
          })
        )
      }
      const state = readCloudflareState()
      const records = Array.isArray(state.dnsByZone?.[zoneId]) ? [...state.dnsByZone[zoneId]] : []
      const index = records.findIndex((entry) => String(entry?.name || '').trim() === name && String(entry?.type || '').trim().toUpperCase() === type)
      const nextRecord = {
        id: index >= 0 ? String(records[index]?.id || `dns-sim-${Date.now()}`) : `dns-sim-${Date.now()}`,
        type,
        name,
        content,
        ttl: Number(body?.ttl || 1) || 1,
        proxied: Boolean(body?.proxied)
      }
      let action = 'created'
      if (index >= 0) {
        action = 'updated'
        records[index] = nextRecord
      } else {
        records.unshift(nextRecord)
      }
      state.dnsByZone = {
        ...(state.dnsByZone || {}),
        [zoneId]: records
      }
      writeCloudflareState(state)
      return Promise.resolve(
        new win.Response(JSON.stringify({ ok: true, zone_id: zoneId, action, record: nextRecord }), {
          status: 200,
          headers: { 'content-type': 'application/json; charset=utf-8' }
        })
      )
    }
    if (pathname === '/v1/local/mcp/integration/start' && String(init?.method || 'GET').toUpperCase() === 'POST') {
      const body = JSON.parse(String(init?.body || '{}'))
      const nodeError = rejectIfNodeIdProvided(body?.node_id)
      if (nodeError) return Promise.resolve(nodeError)
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
      const nodeError = rejectIfNodeIdProvided(body?.node_id)
      if (nodeError) return Promise.resolve(nodeError)
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
      const requestedNodeId = (() => {
        try {
          return String(new URL(url, win.location.origin).searchParams.get('node_id') || '').trim()
        } catch {
          return ''
        }
      })()
      const nodeError = rejectIfNodeIdProvided(requestedNodeId)
      if (nodeError) return Promise.resolve(nodeError)
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
