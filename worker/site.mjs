// SPDX-License-Identifier: Apache-2.0

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    if (url.pathname === '/api/lead') {
      return handleLeadCollection(request, env)
    }
    return env.ASSETS.fetch(request)
  }
}

async function handleLeadCollection(request, env) {
  if (request.method !== 'POST') {
    return jsonResponse({ error: 'method_not_allowed' }, 405)
  }
  if (!env.EMAIL_SIGNUPS || typeof env.EMAIL_SIGNUPS.put !== 'function') {
    return jsonResponse({ error: 'storage_unavailable' }, 503)
  }

  const payload = await parsePayload(request)
  const normalizedEmail = typeof payload.email === 'string' ? payload.email.trim().toLowerCase() : ''
  if (!EMAIL_RE.test(normalizedEmail)) {
    return jsonResponse({ error: 'invalid_email' }, 400)
  }

  const sourcePath = normalizeSourcePath(payload.sourcePath)
  const submittedAt = new Date().toISOString()
  const digest = await sha256Hex(normalizedEmail)
  const record = {
    email: normalizedEmail,
    sourcePath,
    submittedAt,
    userAgent: request.headers.get('user-agent') || '',
    country: request.cf?.country || null
  }

  await env.EMAIL_SIGNUPS.put(`lead:${digest}`, JSON.stringify(record))
  return jsonResponse({ ok: true }, 202)
}

async function parsePayload(request) {
  const contentType = (request.headers.get('content-type') || '').toLowerCase()
  if (contentType.includes('application/json')) {
    try {
      return await request.json()
    } catch {
      return {}
    }
  }
  if (contentType.includes('application/x-www-form-urlencoded') || contentType.includes('multipart/form-data')) {
    const formData = await request.formData()
    return {
      email: formData.get('email'),
      sourcePath: formData.get('sourcePath')
    }
  }
  return {}
}

function normalizeSourcePath(value) {
  if (typeof value !== 'string') return '/'
  const trimmed = value.trim()
  if (!trimmed.startsWith('/')) return '/'
  return trimmed.slice(0, 160)
}

async function sha256Hex(value) {
  const bytes = new TextEncoder().encode(value)
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return [...new Uint8Array(digest)].map((part) => part.toString(16).padStart(2, '0')).join('')
}

function jsonResponse(body, status) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      'cache-control': 'no-store',
      'content-type': 'application/json; charset=utf-8'
    }
  })
}
