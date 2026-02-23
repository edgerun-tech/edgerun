/**
 * IndexedDB File System
 * Persistent file storage in browser
 */

const DB_NAME = 'browser-os-fs';
const DB_VERSION = 1;
const STORE_NAME = 'files';

export interface FSFile {
  path: string;
  content: string;
  type: 'file' | 'folder';
  size: number;
  created: number;
  modified: number;
}

class IndexedDBFS {
  private db: IDBDatabase | null = null;
  private initPromise: Promise<void> | null = null;

  async init(): Promise<void> {
    if (this.db) return;
    if (this.initPromise) return this.initPromise;

    this.initPromise = new Promise((resolve, reject) => {
      const request = indexedDB.open(DB_NAME, DB_VERSION);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        this.db = request.result;
        resolve();
      };

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          db.createObjectStore(STORE_NAME, { keyPath: 'path' });
        }
      };
    });

    return this.initPromise;
  }

  async writeFile(path: string, content: string): Promise<void> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);

      const now = Date.now();
      const file: FSFile = {
        path,
        content,
        type: 'file',
        size: content.length,
        created: now,
        modified: now,
      };

      // Also create parent directories
      const parts = path.split('/').slice(0, -1);
      let currentPath = '';
      for (const part of parts) {
        if (!part) continue;
        currentPath += '/' + part;
        store.get(currentPath).onsuccess = (e) => {
          if (!e.target.result) {
            store.put({
              path: currentPath,
              content: '',
              type: 'folder',
              size: 0,
              created: now,
              modified: now,
            });
          }
        };
      }

      const putRequest = store.put(file);
      putRequest.onsuccess = () => resolve();
      putRequest.onerror = () => reject(putRequest.error);
    });
  }

  async readFile(path: string): Promise<string | null> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.get(path);

      request.onsuccess = () => {
        const result = request.result as FSFile | undefined;
        resolve(result?.content ?? null);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async deleteFile(path: string): Promise<void> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      const request = store.delete(path);

      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async listFiles(dirPath: string): Promise<FSFile[]> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.getAll();

      request.onsuccess = () => {
        const allFiles = request.result as FSFile[];
        const normalizedDir = dirPath.endsWith('/') ? dirPath : dirPath + '/';
        
        const filesInDir = allFiles.filter(f => {
          if (f.path === dirPath) return false;
          if (!f.path.startsWith(normalizedDir)) return false;
          // Only immediate children
          const relative = f.path.replace(normalizedDir, '');
          return !relative.includes('/');
        });

        // Also include folders
        const folders = allFiles.filter(f => 
          f.type === 'folder' && 
          f.path.startsWith(normalizedDir) &&
          f.path !== dirPath
        );

        resolve([...filesInDir, ...folders]);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async searchFiles(query: string, limit: number = 20): Promise<FSFile[]> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.getAll();

      request.onsuccess = () => {
        const allFiles = request.result as FSFile[];
        const queryLower = query.toLowerCase();
        
        const matches = allFiles
          .filter(f => f.type === 'file' && f.path.toLowerCase().includes(queryLower))
          .slice(0, limit);

        resolve(matches);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async fileExists(path: string): Promise<boolean> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.get(path);

      request.onsuccess = () => {
        resolve(!!request.result);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async createFolder(path: string): Promise<void> {
    await this.init();
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error('DB not initialized'));
        return;
      }

      const tx = this.db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      const now = Date.now();

      const folder: FSFile = {
        path,
        content: '',
        type: 'folder',
        size: 0,
        created: now,
        modified: now,
      };

      const request = store.put(folder);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }
}

export const indexedDBFS = new IndexedDBFS();
