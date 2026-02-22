// SPDX-License-Identifier: Apache-2.0
export function createIndexedDbAdapter({ dbName, storeName, key }) {
  function fallbackGet() {
    try {
      const raw = localStorage.getItem(key);
      return raw ? JSON.parse(raw) : null;
    } catch {
      return null;
    }
  }

  function fallbackSet(value) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {}
  }

  function openDb() {
    return new Promise((resolve, reject) => {
      if (!('indexedDB' in window)) {
        reject(new Error('indexeddb unavailable'));
        return;
      }
      const request = indexedDB.open(dbName, 1);
      request.onupgradeneeded = () => {
        const db = request.result;
        if (!db.objectStoreNames.contains(storeName)) {
          db.createObjectStore(storeName);
        }
      };
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error || new Error('indexeddb open failed'));
    });
  }

  async function idbGet() {
    const db = await openDb();
    try {
      return await new Promise((resolve, reject) => {
        const tx = db.transaction(storeName, 'readonly');
        const store = tx.objectStore(storeName);
        const req = store.get(key);
        req.onsuccess = () => resolve(req.result ?? null);
        req.onerror = () => reject(req.error || new Error('indexeddb read failed'));
      });
    } finally {
      db.close();
    }
  }

  async function idbSet(value) {
    const db = await openDb();
    try {
      await new Promise((resolve, reject) => {
        const tx = db.transaction(storeName, 'readwrite');
        const store = tx.objectStore(storeName);
        const req = store.put(value, key);
        req.onsuccess = () => resolve();
        req.onerror = () => reject(req.error || new Error('indexeddb write failed'));
      });
    } finally {
      db.close();
    }
  }

  return {
    async get() {
      try {
        return await idbGet();
      } catch {
        return fallbackGet();
      }
    },
    async set(value) {
      try {
        await idbSet(value);
      } catch {
        fallbackSet(value);
      }
    },
  };
}
