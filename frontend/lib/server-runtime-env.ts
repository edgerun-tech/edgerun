import { getCloudflareContext } from "@opennextjs/cloudflare"

type SecretBinding = {
  get?: () => Promise<string>
}

export async function runtimeEnv(name: string): Promise<string | undefined> {
  const processValue = process.env[name]
  if (processValue) return processValue

  try {
    const context = await getCloudflareContext({ async: true })
    const binding = (context.env as Record<string, unknown>)[name] as string | SecretBinding | undefined
    if (typeof binding === "string") return binding
    if (binding?.get) return await binding.get()
  } catch {
    return undefined
  }

  return undefined
}

export async function runtimeEnvAny(...names: string[]): Promise<string | undefined> {
  for (const name of names) {
    const value = await runtimeEnv(name)
    if (value) return value
  }
  return undefined
}
