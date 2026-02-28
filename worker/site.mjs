// SPDX-License-Identifier: Apache-2.0

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    if (url.pathname.startsWith('/api/')) {
      return new Response('gone', { status: 410 })
    }
    return env.ASSETS.fetch(request)
  }
}
