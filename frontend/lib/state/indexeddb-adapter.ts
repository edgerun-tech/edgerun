export type IndexedDbAdapterOptions = {
  dbName: string
  storeName: string
  key: string
}

export type IndexedDbAdapter<T> = {
  get: () => Promise<T | null>
  set: (value: T) => Promise<void>
}

export function createIndexedDbAdapter<T>(options: IndexedDbAdapterOptions): IndexedDbAdapter<T> {
  const { dbName, storeName, key } = options

  function fallbackGet(): T | null {
    if (typeof window === 'undefined') return null
    try {
      const raw = window.localStorage.getItem(key)
      if (!raw) return null
      return JSON.parse(raw) as T
    } catch {
      return null
    }
  }

  function fallbackSet(value: T): void {
    if (typeof window === 'undefined') return
    try {
      window.localStorage.setItem(key, JSON.stringify(value))
    } catch {
      // ignore persistence failures
    }
  }

  function openDb(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      if (typeof window === 'undefined' || !('indexedDB' in window)) {
        reject(new Error('indexeddb unavailable'))
        return
      }

      const request = window.indexedDB.open(dbName, 1)
      request.onupgradeneeded = () => {
        const db = request.result
        if (!db.objectStoreNames.contains(storeName)) {
          db.createObjectStore(storeName)
        }
      }
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error ?? new Error('indexeddb open failed'))
    })
  }

  async function idbGet(): Promise<T | null> {
    const db = await openDb()
    try {
      return await new Promise<T | null>((resolve, reject) => {
        const tx = db.transaction(storeName, 'readonly')
        const store = tx.objectStore(storeName)
        const req = store.get(key)
        req.onsuccess = () => resolve((req.result as T | undefined) ?? null)
        req.onerror = () => reject(req.error ?? new Error('indexeddb read failed'))
      })
    } finally {
      db.close()
    }
  }

  async function idbSet(value: T): Promise<void> {
    const db = await openDb()
    try {
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction(storeName, 'readwrite')
        const store = tx.objectStore(storeName)
        const req = store.put(value, key)
        req.onsuccess = () => resolve()
        req.onerror = () => reject(req.error ?? new Error('indexeddb write failed'))
      })
    } finally {
      db.close()
    }
  }

  return {
    async get() {
      try {
        return await idbGet()
      } catch {
        return fallbackGet()
      }
    },
    async set(value: T) {
      try {
        await idbSet(value)
      } catch {
        fallbackSet(value)
      }
    }
  }
}
