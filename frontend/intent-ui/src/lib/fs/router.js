import { ramfsProvider } from "./providers/ramfs";
import { githubProvider } from "./providers/github";
import { localProvider } from "./providers/local";
import { gdriveProvider } from "./providers/gdrive";

const providers = {
  ramfs: ramfsProvider,
  github: githubProvider,
  local: localProvider,
  gdrive: gdriveProvider
};

const mounts = [
  { id: "ram", providerId: "ramfs", label: "RAMFS", root: "/ram" },
  { id: "github", providerId: "github", label: "GitHub", root: "/github" },
  { id: "local", providerId: "local", label: "Local", root: "/local" },
  { id: "drive", providerId: "gdrive", label: "Google Drive", root: "/drive" }
];

function normalize(path) {
  if (!path) return "/";
  let next = path.startsWith("/") ? path : `/${path}`;
  next = next.replace(/\/{2,}/g, "/");
  if (next.length > 1 && next.endsWith("/")) next = next.slice(0, -1);
  return next;
}

function resolveMount(path) {
  const normalized = normalize(path);
  const mount = mounts.find((candidate) => normalized === candidate.root || normalized.startsWith(`${candidate.root}/`));
  if (!mount) return null;
  const provider = providers[mount.providerId];
  if (!provider) return null;
  const providerPath = normalized === mount.root ? "/" : normalized.slice(mount.root.length) || "/";
  return { mount, provider, providerPath: normalize(providerPath) };
}

const fsRouter = {
  getMounts() {
    return mounts.map((mount) => ({
      id: mount.id,
      root: mount.root,
      label: mount.label,
      providerId: mount.providerId,
      auth: providers[mount.providerId]?.authState?.() || "error"
    }));
  },
  resolve(path) {
    return resolveMount(path);
  },
  async list(path) {
    const normalized = normalize(path);
    if (normalized === "/") {
      return this.getMounts().map((mount) => ({
        id: `mount:${mount.id}`,
        provider: mount.providerId,
        mountId: mount.id,
        path: mount.root,
        name: mount.label,
        kind: "dir",
        auth: mount.auth
      }));
    }
    const resolved = resolveMount(normalized);
    if (!resolved) {
      throw new Error(`No mounted provider for path: ${normalized}`);
    }
    const entries = await resolved.provider.list(resolved.providerPath);
    return entries.map((entry) => ({
      ...entry,
      mountId: resolved.mount.id,
      auth: resolved.provider.authState?.() || "error",
      path: `${resolved.mount.root}${entry.path === "/" ? "" : entry.path}`
    }));
  },
  async read(path) {
    const resolved = resolveMount(path);
    if (!resolved) throw new Error(`No mounted provider for path: ${path}`);
    return resolved.provider.read(resolved.providerPath);
  },
  async write(path, content) {
    const resolved = resolveMount(path);
    if (!resolved) throw new Error(`No mounted provider for path: ${path}`);
    return resolved.provider.write(resolved.providerPath, content);
  },
  async mkdir(path) {
    const resolved = resolveMount(path);
    if (!resolved) throw new Error(`No mounted provider for path: ${path}`);
    return resolved.provider.mkdir(resolved.providerPath);
  },
  async delete(path) {
    const resolved = resolveMount(path);
    if (!resolved) throw new Error(`No mounted provider for path: ${path}`);
    return resolved.provider.delete(resolved.providerPath);
  },
  async move(from, to) {
    const fromResolved = resolveMount(from);
    const toResolved = resolveMount(to);
    if (!fromResolved || !toResolved || fromResolved.mount.id !== toResolved.mount.id) {
      throw new Error("Cross-provider move is not supported yet.");
    }
    return fromResolved.provider.move(fromResolved.providerPath, toResolved.providerPath);
  },
  async copy(from, to) {
    const fromResolved = resolveMount(from);
    const toResolved = resolveMount(to);
    if (!fromResolved || !toResolved || fromResolved.mount.id !== toResolved.mount.id) {
      throw new Error("Cross-provider copy is not supported yet.");
    }
    if (typeof fromResolved.provider.copy === "function") {
      return fromResolved.provider.copy(fromResolved.providerPath, toResolved.providerPath);
    }
    const content = await fromResolved.provider.read(fromResolved.providerPath);
    return toResolved.provider.write(toResolved.providerPath, content);
  }
};

export { fsRouter };
