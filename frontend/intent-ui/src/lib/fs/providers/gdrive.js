const FOLDER_MIME = "application/vnd.google-apps.folder";
const DOC_MIME = "application/vnd.google-apps.document";
const GOOGLE_TOKEN_KEY = "google_token";
const GOOGLE_REFRESH_KEY = "google_refresh";
const GOOGLE_EXPIRES_KEY = "google_token_expires_at";

const pathToIdCache = new Map([["/", "root"]]);

function normalize(path) {
  if (!path) return "/";
  let next = path.startsWith("/") ? path : `/${path}`;
  next = next.replace(/\/{2,}/g, "/");
  if (next.length > 1 && next.endsWith("/")) next = next.slice(0, -1);
  return next;
}

function getToken() {
  if (typeof localStorage === "undefined") return "";
  return String(localStorage.getItem(GOOGLE_TOKEN_KEY) || "").trim();
}

function getRefreshToken() {
  if (typeof localStorage === "undefined") return "";
  return String(localStorage.getItem(GOOGLE_REFRESH_KEY) || "").trim();
}

function getExpiresAt() {
  if (typeof localStorage === "undefined") return 0;
  const raw = Number(localStorage.getItem(GOOGLE_EXPIRES_KEY) || 0);
  return Number.isFinite(raw) ? raw : 0;
}

async function refreshTokenIfNeeded(force = false) {
  if (typeof localStorage === "undefined") return "";
  const current = getToken();
  const refresh = getRefreshToken();
  const expiresAt = getExpiresAt();
  const expiringSoon = expiresAt > 0 && expiresAt <= Date.now() + 30 * 1000;
  if (!refresh || (!force && current && !expiringSoon)) {
    return current;
  }
  const response = await fetch("/api/google/refresh", {
    method: "POST",
    headers: { "content-type": "application/json; charset=utf-8" },
    body: JSON.stringify({ refresh_token: refresh })
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || !payload?.ok || !payload?.access_token) {
    throw new Error(payload?.error || "Google token refresh failed.");
  }
  const nextToken = String(payload.access_token || "").trim();
  if (!nextToken) throw new Error("Google token refresh failed.");
  localStorage.setItem(GOOGLE_TOKEN_KEY, nextToken);
  if (payload.expires_at) {
    localStorage.setItem(GOOGLE_EXPIRES_KEY, String(payload.expires_at));
  }
  return nextToken;
}

async function api(pathname, query = {}) {
  let token = await refreshTokenIfNeeded(false);
  if (!token) throw new Error("Google token missing. Link Google integration.");
  const send = async (activeToken) => {
    const url = new URL(pathname, window.location.origin);
    url.searchParams.set("token", activeToken);
    for (const [key, value] of Object.entries(query)) {
      if (value === undefined || value === null || value === "") continue;
      url.searchParams.set(key, String(value));
    }
    const response = await fetch(url.toString());
    const payload = await response.json().catch(() => ({}));
    return { response, payload };
  };
  let { response, payload } = await send(token);
  if (response.status === 401 && getRefreshToken()) {
    token = await refreshTokenIfNeeded(true);
    ({ response, payload } = await send(token));
  }
  if (!response.ok || payload?.ok === false) {
    throw new Error(payload?.error || `Google Drive request failed (${response.status})`);
  }
  return payload;
}

async function listChildren(parentId) {
  const payload = await api("/api/google/drive/files", { parentId, pageSize: 200 });
  return Array.isArray(payload?.files) ? payload.files : [];
}

async function resolvePathId(path) {
  const normalized = normalize(path);
  if (pathToIdCache.has(normalized)) {
    return pathToIdCache.get(normalized);
  }
  const segments = normalized.split("/").filter(Boolean);
  let currentPath = "/";
  let currentId = "root";
  for (const segment of segments) {
    const childPath = normalize(`${currentPath}/${segment}`);
    if (pathToIdCache.has(childPath)) {
      currentId = pathToIdCache.get(childPath);
      currentPath = childPath;
      continue;
    }
    const children = await listChildren(currentId);
    const matched = children.find((item) => item?.name === segment);
    if (!matched?.id) {
      return "";
    }
    currentId = matched.id;
    currentPath = childPath;
    pathToIdCache.set(currentPath, currentId);
  }
  return currentId;
}

const gdriveProvider = {
  id: "gdrive",
  label: "Google Drive",
  authState() {
    return getToken() ? "ready" : "needs-auth";
  },
  async list(path) {
    const normalized = normalize(path);
    const parentId = await resolvePathId(normalized);
    if (!parentId) return [];
    const files = await listChildren(parentId);
    return files.map((file) => {
      const childPath = normalize(`${normalized}/${file.name}`);
      pathToIdCache.set(childPath, file.id);
      return {
        id: `gdrive:${file.id}`,
        provider: "gdrive",
        path: childPath,
        name: file.name,
        kind: file.mimeType === FOLDER_MIME ? "dir" : "file",
        size: typeof file.size === "string" ? Number(file.size) : undefined
      };
    }).sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
  },
  async read(path) {
    const normalized = normalize(path);
    const fileId = await resolvePathId(normalized);
    if (!fileId || fileId === "root") {
      throw new Error("Google Drive file not found.");
    }
    const payload = await api(`/api/google/drive/file/${encodeURIComponent(fileId)}`);
    const mimeType = String(payload?.file?.mimeType || "");
    if (mimeType === FOLDER_MIME) {
      throw new Error("Cannot open a folder.");
    }
    if (mimeType === DOC_MIME && !payload?.content) {
      throw new Error("Google Docs export is not available in this build.");
    }
    return String(payload?.content || "");
  },
  async write() {
    throw new Error("Google Drive write is not implemented yet.");
  },
  async mkdir() {
    throw new Error("Google Drive mkdir is not implemented yet.");
  },
  async delete() {
    throw new Error("Google Drive delete is not implemented yet.");
  },
  async move() {
    throw new Error("Google Drive move is not implemented yet.");
  }
};

export { gdriveProvider };
