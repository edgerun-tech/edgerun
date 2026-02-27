const RAMFS_KEY = "intent-ui-ramfs-v1";

const seed = {
  "/README.md": "# IntentUI RAMFS\n\nThis file lives in browser RAMFS.",
  "/src/App.jsx": "export default function App() {\n  return <div>Hello from RAMFS</div>\n}\n",
  "/src/components/panels/IntentBar.jsx": "// IntentBar source placeholder\n",
  "/src/stores/windows.js": "// windows store placeholder\n",
  "/package.json": "{\n  \"name\": \"intent-ui\"\n}\n"
};

function normalize(path) {
  if (!path) return "/";
  let next = path.startsWith("/") ? path : `/${path}`;
  next = next.replace(/\/{2,}/g, "/");
  if (next.length > 1 && next.endsWith("/")) next = next.slice(0, -1);
  return next;
}

function parentDir(path) {
  const normalized = normalize(path);
  if (normalized === "/") return "/";
  const idx = normalized.lastIndexOf("/");
  return idx <= 0 ? "/" : normalized.slice(0, idx);
}

function basename(path) {
  const normalized = normalize(path);
  if (normalized === "/") return "/";
  const idx = normalized.lastIndexOf("/");
  return normalized.slice(idx + 1);
}

function readStore() {
  if (typeof localStorage === "undefined") return { ...seed };
  try {
    const raw = localStorage.getItem(RAMFS_KEY);
    if (!raw) return { ...seed };
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return { ...seed };
    return parsed;
  } catch {
    return { ...seed };
  }
}

function writeStore(store) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(RAMFS_KEY, JSON.stringify(store));
  } catch {
    // ignore quota/storage errors
  }
}

function listEntries(store, path) {
  const dir = normalize(path);
  const childMap = /* @__PURE__ */ new Map();
  const prefix = dir === "/" ? "/" : `${dir}/`;
  for (const fullPath of Object.keys(store)) {
    const normalized = normalize(fullPath);
    if (dir !== "/" && !normalized.startsWith(prefix)) continue;
    if (dir === "/" && !normalized.startsWith("/")) continue;
    if (normalized === dir) continue;
    const rest = dir === "/" ? normalized.slice(1) : normalized.slice(prefix.length);
    if (!rest) continue;
    const parts = rest.split("/");
    const name = parts[0];
    const childPath = normalize(dir === "/" ? `/${name}` : `${dir}/${name}`);
    const isDir = parts.length > 1;
    const existing = childMap.get(childPath);
    if (!existing) {
      childMap.set(childPath, {
        id: `ramfs:${childPath}`,
        provider: "ramfs",
        path: childPath,
        name,
        kind: isDir ? "dir" : "file",
        size: isDir ? undefined : String(store[normalized] || "").length
      });
    } else if (isDir && existing.kind !== "dir") {
      existing.kind = "dir";
      existing.size = undefined;
    }
  }
  return Array.from(childMap.values()).sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

const ramfsProvider = {
  id: "ramfs",
  label: "Browser RAMFS",
  authState() {
    return "ready";
  },
  async list(path) {
    const store = readStore();
    return listEntries(store, path);
  },
  async read(path) {
    const store = readStore();
    const normalized = normalize(path);
    if (!(normalized in store)) {
      throw new Error(`File not found: ${normalized}`);
    }
    return String(store[normalized] ?? "");
  },
  async write(path, content) {
    const store = readStore();
    const normalized = normalize(path);
    const parent = parentDir(normalized);
    const entries = listEntries(store, parent);
    const parentExists = parent === "/" || entries.some((entry) => entry.path === parent && entry.kind === "dir") || Object.keys(store).some((p) => normalize(p).startsWith(`${parent}/`));
    if (!parentExists) {
      throw new Error(`Parent directory does not exist: ${parent}`);
    }
    store[normalized] = typeof content === "string" ? content : String(content);
    writeStore(store);
  },
  async mkdir(path) {
    const normalized = normalize(path);
    const markerPath = normalize(`${normalized}/.keep`);
    const store = readStore();
    if (!(markerPath in store)) {
      store[markerPath] = "";
      writeStore(store);
    }
  },
  async delete(path) {
    const normalized = normalize(path);
    const store = readStore();
    for (const key of Object.keys(store)) {
      const candidate = normalize(key);
      if (candidate === normalized || candidate.startsWith(`${normalized}/`)) {
        delete store[key];
      }
    }
    writeStore(store);
  },
  async move(from, to) {
    const fromPath = normalize(from);
    const toPath = normalize(to);
    const store = readStore();
    const dirKeys = Object.keys(store).filter((key) => {
      const candidate = normalize(key);
      return candidate.startsWith(`${fromPath}/`);
    });
    if (dirKeys.length > 0) {
      for (const key of dirKeys) {
        const candidate = normalize(key);
        const suffix = candidate.slice(fromPath.length);
        const nextPath = normalize(`${toPath}${suffix}`);
        store[nextPath] = store[key];
        delete store[key];
      }
      writeStore(store);
      return;
    }
    const value = store[fromPath];
    if (typeof value === "undefined") throw new Error(`File not found: ${fromPath}`);
    delete store[fromPath];
    store[toPath] = value;
    writeStore(store);
  },
  async copy(from, to) {
    const fromPath = normalize(from);
    const toPath = normalize(to);
    const store = readStore();
    const dirKeys = Object.keys(store).filter((key) => {
      const candidate = normalize(key);
      return candidate.startsWith(`${fromPath}/`);
    });
    if (dirKeys.length > 0) {
      for (const key of dirKeys) {
        const candidate = normalize(key);
        const suffix = candidate.slice(fromPath.length);
        const nextPath = normalize(`${toPath}${suffix}`);
        store[nextPath] = store[key];
      }
      writeStore(store);
      return;
    }
    const value = store[fromPath];
    if (typeof value === "undefined") throw new Error(`File not found: ${fromPath}`);
    store[toPath] = value;
    writeStore(store);
  }
};

export { ramfsProvider };
