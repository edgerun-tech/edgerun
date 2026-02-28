// SPDX-License-Identifier: Apache-2.0

const CONTROL_PREFIXES = [
  '/intent/',
  '/intent-ui/',
  '/assets/',
  '/fonts/',
  '/favicon.ico',
  '/icon.svg',
  '/apple-icon.png',
  '/manifest.webmanifest',
  '/robots.txt',
  '/sitemap.xml'
]

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    if (url.pathname === '/api/tunnel/reserve-domain' && request.method === 'POST') {
      return handleReserveDomain(request, env)
    }
    if (url.pathname === '/api/tunnel/create-pairing-code' && request.method === 'POST') {
      return handleCreatePairingCode(request, env)
    }
    if (url.pathname === '/') {
      const rootRequest = new Request(new URL('/intent-ui/', url), request)
      return env.ASSETS.fetch(rootRequest)
    }
    if (!isAllowedPath(url.pathname)) {
      return new Response('not_found', { status: 404 })
    }
    return env.ASSETS.fetch(request)
  }
}

function isAllowedPath(pathname) {
  return CONTROL_PREFIXES.some((prefix) => {
    if (prefix.endsWith('/')) return pathname.startsWith(prefix)
    return pathname === prefix
  })
}

async function handleCreatePairingCode(request, env) {
  try {
    const body = await request.json().catch(() => ({}))
    const domain = String(body?.domain || '').trim()
    const registrationToken = String(body?.registrationToken || '').trim()
    const ttlSeconds = Number(body?.ttlSeconds || 300)
    if (!domain || !registrationToken) {
      return json(
        { ok: false, error: 'domain and registrationToken are required' },
        { status: 400 }
      )
    }
    const relayBase = String(env?.RELAY_CONTROL_BASE || 'https://relay.edgerun.tech').replace(/\/+$/, '')
    const payload = encodeCreatePairingCodeRequest({
      domain,
      registrationToken,
      ttlSeconds
    })
    const relayResponse = await fetch(`${relayBase}/v1/tunnel/create-pairing-code`, {
      method: 'POST',
      headers: { 'content-type': 'application/x-protobuf' },
      body: payload
    })
    const bytes = new Uint8Array(await relayResponse.arrayBuffer())
    const decoded = decodeCreatePairingCodeResponse(bytes)
    return json(
      {
        ok: Boolean(decoded.ok),
        error: decoded.error || '',
        pairingCode: decoded.pairingCode || '',
        expiresUnixMs: Number(decoded.expiresUnixMs || 0),
        deviceCommand: decoded.deviceCommand || ''
      },
      { status: relayResponse.ok ? 200 : relayResponse.status || 502 }
    )
  } catch (error) {
    const message = error instanceof Error ? error.message : 'pairing code issuance failed'
    return json({ ok: false, error: message }, { status: 502 })
  }
}

async function handleReserveDomain(request, env) {
  try {
    const body = await request.json().catch(() => ({}))
    const profilePublicKey = String(body?.profilePublicKeyB64url || '').trim()
    const requestedLabel = String(body?.requestedLabel || '').trim()
    if (!profilePublicKey) {
      return json(
        { ok: false, error: 'profilePublicKeyB64url is required' },
        { status: 400 }
      )
    }
    const leaseSecret = String(env?.RELAY_LEASE_HMAC_SECRET || '').trim()
    if (!leaseSecret) {
      return json(
        { ok: false, error: 'RELAY_LEASE_HMAC_SECRET is not configured' },
        { status: 500 }
      )
    }
    const userId = await deriveUserId(profilePublicKey)
    const sanitizedLabel = sanitizeLabel(requestedLabel)
    const label = sanitizedLabel ? `${sanitizedLabel}-${userId.slice(0, 6)}` : userId
    const domain = `${label}.users.edgerun.tech`
    const expiresUnixMs = Date.now() + (15 * 60 * 1000)
    const registrationToken = await signLeaseToken({
      profilePublicKeyB64url: profilePublicKey,
      domain,
      issuedUnixMs: Date.now(),
      expiresUnixMs,
      nonce: randomTokenHex(16)
    }, leaseSecret)
    return json(
      {
        ok: true,
        error: '',
        userId,
        domain,
        status: 'lease_issued',
        registrationToken
      },
      { status: 200 }
    )
  } catch (error) {
    const message = error instanceof Error ? error.message : 'domain reservation failed'
    return json({ ok: false, error: message }, { status: 502 })
  }
}

function json(payload, init = {}) {
  return new Response(JSON.stringify(payload), {
    status: init.status || 200,
    headers: {
      'content-type': 'application/json; charset=utf-8',
      'cache-control': 'no-store'
    }
  })
}

function encodeVarint(value) {
  let n = Number(value || 0)
  const out = []
  while (n >= 0x80) {
    out.push((n & 0x7f) | 0x80)
    n = Math.floor(n / 128)
  }
  out.push(n & 0x7f)
  return out
}

function decodeVarint(bytes, start) {
  let value = 0
  let shift = 0
  let offset = start
  while (offset < bytes.length) {
    const byte = bytes[offset]
    value += (byte & 0x7f) * (2 ** shift)
    offset += 1
    if ((byte & 0x80) === 0) {
      return { value, offset }
    }
    shift += 7
  }
  throw new Error('invalid protobuf varint')
}

function encodeField(tag, wireType, payloadBytes) {
  const key = Uint8Array.from(encodeVarint((tag << 3) | wireType))
  const out = new Uint8Array(key.length + payloadBytes.length)
  out.set(key, 0)
  out.set(payloadBytes, key.length)
  return out
}

function concat(parts) {
  const length = parts.reduce((sum, part) => sum + part.length, 0)
  const out = new Uint8Array(length)
  let offset = 0
  for (const part of parts) {
    out.set(part, offset)
    offset += part.length
  }
  return out
}

function encodeString(tag, value) {
  const bytes = new TextEncoder().encode(String(value || ''))
  return encodeField(tag, 2, concat([Uint8Array.from(encodeVarint(bytes.length)), bytes]))
}

function encodeUInt64(tag, value) {
  return encodeField(tag, 0, Uint8Array.from(encodeVarint(value)))
}

function encodeCreatePairingCodeRequest(input) {
  return concat([
    encodeString(1, input.domain),
    encodeString(2, input.registrationToken),
    encodeUInt64(3, Number(input.ttlSeconds || 300))
  ])
}

function decodeLengthDelimited(bytes, offset) {
  const size = decodeVarint(bytes, offset)
  const end = size.offset + size.value
  if (end > bytes.length) throw new Error('protobuf field overflow')
  return { bytes: bytes.slice(size.offset, end), offset: end }
}

function decodeCreatePairingCodeResponse(bytes) {
  const out = {
    ok: false,
    error: '',
    pairingCode: '',
    expiresUnixMs: 0,
    deviceCommand: ''
  }
  let offset = 0
  while (offset < bytes.length) {
    const key = decodeVarint(bytes, offset)
    offset = key.offset
    const tag = key.value >> 3
    const wire = key.value & 0x07
    if (wire === 2) {
      const field = decodeLengthDelimited(bytes, offset)
      offset = field.offset
      const text = new TextDecoder().decode(field.bytes)
      if (tag === 2) out.error = text
      if (tag === 3) out.pairingCode = text
      if (tag === 5) out.deviceCommand = text
      continue
    }
    if (wire === 0) {
      const value = decodeVarint(bytes, offset)
      offset = value.offset
      if (tag === 1) out.ok = value.value !== 0
      if (tag === 4) out.expiresUnixMs = value.value
      continue
    }
    break
  }
  return out
}

async function deriveUserId(profilePublicKey) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(profilePublicKey))
  const bytes = new Uint8Array(digest)
  return Array.from(bytes.slice(0, 8)).map((n) => n.toString(16).padStart(2, '0')).join('')
}

function sanitizeLabel(raw) {
  return String(raw || '')
    .toLowerCase()
    .split('')
    .filter((ch) => /[a-z0-9-]/.test(ch))
    .join('')
}

function randomTokenHex(size) {
  const bytes = crypto.getRandomValues(new Uint8Array(size))
  return Array.from(bytes).map((n) => n.toString(16).padStart(2, '0')).join('')
}

async function signLeaseToken(claims, secret) {
  const payload = [
    'lease_v1',
    `profile_public_key_b64url=${claims.profilePublicKeyB64url}`,
    `domain=${claims.domain}`,
    `issued_unix_ms=${claims.issuedUnixMs}`,
    `expires_unix_ms=${claims.expiresUnixMs}`,
    `nonce=${claims.nonce}`
  ].join('\n')
  const payloadB64 = base64urlEncode(new TextEncoder().encode(payload))
  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  )
  const signature = new Uint8Array(await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(payloadB64)))
  return `${payloadB64}.${base64urlEncode(signature)}`
}

function base64urlEncode(bytes) {
  let binary = ''
  for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i])
  const base64 = btoa(binary)
  return base64.replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/g, '')
}
