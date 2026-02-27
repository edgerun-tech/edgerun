const localProvider = {
  id: "local",
  label: "Local FS (Forwarded)",
  authState() {
    return "ready";
  },
  async list(path = "/") {
    const response = await fetch(`/api/fs/list?path=${encodeURIComponent(path)}`);
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to list local filesystem.");
    }
    return Array.isArray(payload.entries) ? payload.entries : [];
  },
  async read(path = "/") {
    const response = await fetch(`/api/fs/read?path=${encodeURIComponent(path)}`);
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to read local file.");
    }
    return typeof payload.content === "string" ? payload.content : "";
  },
  async write(path, content) {
    const response = await fetch("/api/fs/write", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ path, content: typeof content === "string" ? content : String(content) })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to write local file.");
    }
  },
  async mkdir(path) {
    const response = await fetch("/api/fs/mkdir", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ path })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to create local directory.");
    }
  },
  async delete(path) {
    const response = await fetch("/api/fs/delete", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ path })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to delete local path.");
    }
  },
  async move(from, to) {
    const response = await fetch("/api/fs/move", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ from, to })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to move local path.");
    }
  },
  async copy(from, to) {
    const response = await fetch("/api/fs/copy", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ from, to })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to copy local path.");
    }
  }
};

export { localProvider };
