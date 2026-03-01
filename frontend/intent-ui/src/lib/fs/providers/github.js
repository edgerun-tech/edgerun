const GITHUB_FS_KEY = "intent-ui-github-fs-v1";
const GITHUB_API_BASE = "https://api.github.com";

const seed = {
  "/README.md": "# GitHub Workspace\n\nThis is a simulated GitHub mount.",
  "/src/App.jsx": "export default function App() {\n  return <main>GitHub mount</main>\n}\n",
  "/src/components/layout/WindowManager.jsx": "// WindowManager demo source\n",
  "/src/components/panels/IntentBar.jsx": "// IntentBar demo source\n"
};

function normalize(path) {
  if (!path) return "/";
  let next = path.startsWith("/") ? path : `/${path}`;
  next = next.replace(/\/{2,}/g, "/");
  if (next.length > 1 && next.endsWith("/")) next = next.slice(0, -1);
  return next;
}

function readStore() {
  if (typeof localStorage === "undefined") return { ...seed };
  try {
    const raw = localStorage.getItem(GITHUB_FS_KEY);
    if (!raw) return { ...seed };
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : { ...seed };
  } catch {
    return { ...seed };
  }
}

function writeStore(store) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(GITHUB_FS_KEY, JSON.stringify(store));
  } catch {
    // ignore
  }
}

function getGitHubToken() {
  if (typeof localStorage === "undefined") return "";
  return String(localStorage.getItem("github_token") || "").trim();
}

function encodePathSegments(path) {
  return path
    .split("/")
    .filter(Boolean)
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

async function githubApi(pathname) {
  const token = getGitHubToken();
  if (!token) {
    throw new Error("GitHub token is missing. Open Integrations and link GitHub.");
  }
  const response = await fetch(`${GITHUB_API_BASE}${pathname}`, {
    headers: {
      accept: "application/vnd.github+json",
      authorization: `Bearer ${token}`,
      "x-github-api-version": "2022-11-28"
    }
  });
  if (!response.ok) {
    const detail = await response.text().catch(() => "");
    throw new Error(`GitHub API request failed (${response.status}): ${detail || response.statusText}`);
  }
  return response.json();
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
    if (!childMap.has(childPath)) {
      childMap.set(childPath, {
        id: `github:${childPath}`,
        provider: "github",
        path: childPath,
        name,
        kind: isDir ? "dir" : "file",
        size: isDir ? undefined : String(store[normalized] || "").length
      });
    } else if (isDir) {
      const current = childMap.get(childPath);
      current.kind = "dir";
      current.size = undefined;
    }
  }
  return Array.from(childMap.values()).sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

const githubProvider = {
  id: "github",
  label: "GitHub",
  authState() {
    return getGitHubToken() ? "ready" : "needs-auth";
  },
  async list(path) {
    if (getGitHubToken()) {
      const normalized = normalize(path);
      if (normalized === "/") {
        const repos = await githubApi("/user/repos?per_page=100&sort=updated");
        const owners = Array.from(new Set(
          (Array.isArray(repos) ? repos : [])
            .map((repo) => repo?.owner?.login)
            .filter((value) => typeof value === "string" && value.length > 0)
        )).sort((a, b) => a.localeCompare(b));
        return owners.map((owner) => ({
          id: `github:${owner}`,
          provider: "github",
          path: `/${owner}`,
          name: owner,
          kind: "dir"
        }));
      }

      const segments = normalized.split("/").filter(Boolean);
      if (segments.length === 1) {
        const owner = segments[0];
        const repos = await githubApi(`/user/repos?per_page=100&sort=updated`);
        return (Array.isArray(repos) ? repos : [])
          .filter((repo) => repo?.owner?.login === owner)
          .map((repo) => ({
            id: `github:${owner}/${repo.name}`,
            provider: "github",
            path: `/${owner}/${repo.name}`,
            name: repo.name,
            kind: "dir"
          }))
          .sort((a, b) => a.name.localeCompare(b.name));
      }

      const owner = segments[0];
      const repo = segments[1];
      const filePath = segments.slice(2).join("/");
      const encoded = filePath ? `/${encodePathSegments(filePath)}` : "";
      const payload = await githubApi(`/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/contents${encoded}`);
      if (!Array.isArray(payload)) {
        return [];
      }
      return payload
        .map((entry) => ({
          id: `github:${entry.path}`,
          provider: "github",
          path: `/${owner}/${repo}/${entry.path}`,
          name: entry.name,
          kind: entry.type === "dir" ? "dir" : "file",
          size: typeof entry.size === "number" ? entry.size : void 0
        }))
        .sort((a, b) => {
          if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
          return a.name.localeCompare(b.name);
        });
    }
    const store = readStore();
    return listEntries(store, path);
  },
  async read(path) {
    if (getGitHubToken()) {
      const normalized = normalize(path);
      const segments = normalized.split("/").filter(Boolean);
      if (segments.length < 3) {
        throw new Error("Select a file inside a repository.");
      }
      const owner = segments[0];
      const repo = segments[1];
      const filePath = segments.slice(2).join("/");
      const payload = await githubApi(
        `/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/contents/${encodePathSegments(filePath)}`
      );
      if (!payload || payload.type !== "file" || typeof payload.content !== "string") {
        throw new Error(`GitHub file not found: ${normalized}`);
      }
      if (payload.encoding === "base64") {
        return atob(payload.content.replace(/\n/g, ""));
      }
      return String(payload.content || "");
    }
    const store = readStore();
    const normalized = normalize(path);
    if (!(normalized in store)) {
      throw new Error(`GitHub file not found: ${normalized}`);
    }
    return String(store[normalized] ?? "");
  },
  async write(path, content) {
    const store = readStore();
    store[normalize(path)] = typeof content === "string" ? content : String(content);
    writeStore(store);
  },
  async mkdir(path) {
    const store = readStore();
    store[normalize(`${path}/.keep`)] = "";
    writeStore(store);
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
    const store = readStore();
    const fromPath = normalize(from);
    const toPath = normalize(to);
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
    if (typeof value === "undefined") throw new Error(`GitHub file not found: ${fromPath}`);
    delete store[fromPath];
    store[toPath] = value;
    writeStore(store);
  },
  async copy(from, to) {
    const store = readStore();
    const fromPath = normalize(from);
    const toPath = normalize(to);
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
    if (typeof value === "undefined") throw new Error(`GitHub file not found: ${fromPath}`);
    store[toPath] = value;
    writeStore(store);
  }
};

export { githubProvider };
