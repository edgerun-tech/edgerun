// SPDX-License-Identifier: Apache-2.0
export function normalizeRoutePath(pathname: string): string {
  const cleaned = pathname.replace(/index\.html$/, '')
  if (!cleaned) return '/'
  return cleaned.endsWith('/') ? cleaned : `${cleaned}/`
}
