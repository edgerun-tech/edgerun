const STORAGE_PREFIX = "edgerun:wasm:"

function appKey(appName: string, key: string): string {
  return `${STORAGE_PREFIX}${appName}:${key}`
}

function appKeys(appName: string): string[] {
  const prefix = appKey(appName, "")
  const keys: string[] = []
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i)
    if (k && k.startsWith(prefix)) {
      keys.push(k.slice(prefix.length))
    }
  }
  return keys
}

export class AppStorage {
  constructor(private appName: string) {}

  set(key: string, value: string) {
    localStorage.setItem(appKey(this.appName, key), value)
  }

  get(key: string): string | null {
    return localStorage.getItem(appKey(this.appName, key))
  }

  remove(key: string) {
    localStorage.removeItem(appKey(this.appName, key))
  }

  keys(): string[] {
    return appKeys(this.appName)
  }

  clear() {
    for (const k of this.keys()) {
      localStorage.removeItem(appKey(this.appName, k))
    }
  }

  setBytes(key: string, bytes: Uint8Array) {
    const b64 = btoa(String.fromCharCode(...bytes))
    this.set(key, `b64:${b64}`)
  }

  getBytes(key: string): Uint8Array | null {
    const val = this.get(key)
    if (!val) return null
    if (!val.startsWith("b64:")) return null
    const b64 = val.slice(4)
    const binary = atob(b64)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i)
    }
    return bytes
  }

  usage(): number {
    let total = 0
    for (const k of this.keys()) {
      const val = this.get(k)
      if (val) total += val.length
    }
    return total
  }
}

export function getAppStorage(appName: string): AppStorage {
  return new AppStorage(appName)
}

export function listAllAppStorages(): { name: string; keys: string[]; usage: number }[] {
  const apps = new Map<string, string[]>()
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i)
    if (k && k.startsWith(STORAGE_PREFIX)) {
      const parts = k.slice(STORAGE_PREFIX.length).split(":")
      const appName = parts[0]
      if (!apps.has(appName)) apps.set(appName, [])
      apps.get(appName)!.push(parts.slice(1).join(":"))
    }
  }
  const result: { name: string; keys: string[]; usage: number }[] = []
  for (const [name, keys] of apps) {
    const storage = new AppStorage(name)
    result.push({ name, keys, usage: storage.usage() })
  }
  return result
}
