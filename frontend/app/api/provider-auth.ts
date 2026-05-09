import { NextRequest } from "next/server"

export function bearerToken(req: NextRequest): string | null {
  const authorization = req.headers.get("authorization")
  if (!authorization) return null
  const [scheme, ...parts] = authorization.split(" ")
  if (scheme.toLowerCase() !== "bearer") return null
  const token = parts.join(" ").trim()
  return token || null
}

export function mediatedProviderToken(req: NextRequest, legacyCookieName: string): string | null {
  return bearerToken(req) ?? req.cookies.get(legacyCookieName)?.value ?? null
}
