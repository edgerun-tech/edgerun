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
    if (url.pathname.startsWith('/api/')) {
      return new Response('gone', { status: 410 })
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
