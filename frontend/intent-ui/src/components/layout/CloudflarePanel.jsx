import { createMemo, createSignal, For, Show, onMount } from "solid-js";
import { FiExternalLink, FiRefreshCw, FiSave } from "solid-icons/fi";
import { integrationStore } from "../../stores/integrations";
import { localBridgeHttpUrl } from "../../lib/local-bridge-origin";

function tokenValue() {
  if (typeof window === "undefined") return "";
  return String(integrationStore.getToken("cloudflare") || localStorage.getItem("cloudflare_token") || "").trim();
}

async function readJson(response) {
  return response.json().catch(() => ({}));
}

function zoneLabel(zone) {
  const name = String(zone?.name || "").trim();
  const status = String(zone?.status || "").trim();
  return status ? `${name} (${status})` : name;
}

export default function CloudflarePanel() {
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");
  const [notice, setNotice] = createSignal("");
  const [zones, setZones] = createSignal([]);
  const [tunnels, setTunnels] = createSignal([]);
  const [apps, setApps] = createSignal([]);
  const [workers, setWorkers] = createSignal([]);
  const [pages, setPages] = createSignal([]);
  const [records, setRecords] = createSignal([]);
  const [accountId, setAccountId] = createSignal("");
  const [zoneId, setZoneId] = createSignal("");
  const [form, setForm] = createSignal({
    type: "CNAME",
    name: "",
    content: "",
    ttl: "1",
    proxied: false
  });

  const hasToken = createMemo(() => tokenValue().length >= 20);

  const loadZones = async () => {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/cloudflare/zones?token=${encodeURIComponent(tokenValue())}`), {
      cache: "no-store"
    });
    const payload = await readJson(response);
    if (!response.ok || payload?.ok === false) {
      throw new Error(String(payload?.error || `cloudflare zones request failed (${response.status})`));
    }
    const next = Array.isArray(payload?.zones) ? payload.zones : [];
    setZones(next);
    if (!zoneId() && next.length > 0) {
      setZoneId(String(next[0]?.id || "").trim());
    }
  };

  const loadTunnels = async () => {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/cloudflare/tunnels?token=${encodeURIComponent(tokenValue())}`), {
      cache: "no-store"
    });
    const payload = await readJson(response);
    if (!response.ok || payload?.ok === false) {
      throw new Error(String(payload?.error || `cloudflare tunnels request failed (${response.status})`));
    }
    setTunnels(Array.isArray(payload?.tunnels) ? payload.tunnels : []);
    setAccountId(String(payload?.account_id || "").trim());
  };

  const loadAccessApps = async () => {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/cloudflare/access/apps?token=${encodeURIComponent(tokenValue())}`), {
      cache: "no-store"
    });
    const payload = await readJson(response);
    if (!response.ok || payload?.ok === false) {
      throw new Error(String(payload?.error || `cloudflare access apps request failed (${response.status})`));
    }
    setApps(Array.isArray(payload?.apps) ? payload.apps : []);
  };

  const loadWorkers = async () => {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/cloudflare/workers?token=${encodeURIComponent(tokenValue())}`), {
      cache: "no-store"
    });
    const payload = await readJson(response);
    if (!response.ok || payload?.ok === false) {
      throw new Error(String(payload?.error || `cloudflare workers request failed (${response.status})`));
    }
    setWorkers(Array.isArray(payload?.workers) ? payload.workers : []);
  };

  const loadPages = async () => {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/cloudflare/pages?token=${encodeURIComponent(tokenValue())}`), {
      cache: "no-store"
    });
    const payload = await readJson(response);
    if (!response.ok || payload?.ok === false) {
      throw new Error(String(payload?.error || `cloudflare pages request failed (${response.status})`));
    }
    setPages(Array.isArray(payload?.pages) ? payload.pages : []);
  };

  const loadDnsRecords = async (selectedZone = zoneId()) => {
    const activeZoneId = String(selectedZone || "").trim();
    if (!activeZoneId) {
      setRecords([]);
      return;
    }
    const response = await fetch(localBridgeHttpUrl(
      `/v1/local/cloudflare/dns/records?token=${encodeURIComponent(tokenValue())}&zone_id=${encodeURIComponent(activeZoneId)}&limit=100`
    ), {
      cache: "no-store"
    });
    const payload = await readJson(response);
    if (!response.ok || payload?.ok === false) {
      throw new Error(String(payload?.error || `cloudflare dns records request failed (${response.status})`));
    }
    setRecords(Array.isArray(payload?.records) ? payload.records : []);
  };

  const refreshAll = async () => {
    if (!hasToken()) {
      setZones([]);
      setTunnels([]);
      setApps([]);
      setWorkers([]);
      setPages([]);
      setRecords([]);
      setError("Cloudflare account API token missing. Connect Cloudflare integration first.");
      return;
    }
    setLoading(true);
    setError("");
    setNotice("");
    try {
      await loadZones();
      await Promise.all([loadTunnels(), loadAccessApps(), loadWorkers(), loadPages()]);
      await loadDnsRecords();
    } catch (refreshError) {
      setError(refreshError instanceof Error ? refreshError.message : "Failed to load Cloudflare resources.");
    } finally {
      setLoading(false);
    }
  };

  const submitDnsUpsert = async () => {
    const activeZoneId = String(zoneId() || "").trim();
    const current = form();
    if (!activeZoneId) {
      setError("Select a domain before creating DNS records.");
      return;
    }
    if (!current.name.trim() || !current.content.trim()) {
      setError("DNS name and content are required.");
      return;
    }
    setLoading(true);
    setError("");
    setNotice("");
    try {
      const response = await fetch(localBridgeHttpUrl("/v1/local/cloudflare/dns/records/upsert"), {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({
          token: tokenValue(),
          zone_id: activeZoneId,
          record_type: current.type,
          name: current.name.trim(),
          content: current.content.trim(),
          ttl: Number(current.ttl || "1") || 1,
          proxied: Boolean(current.proxied)
        })
      });
      const payload = await readJson(response);
      if (!response.ok || payload?.ok === false) {
        throw new Error(String(payload?.error || `cloudflare dns upsert failed (${response.status})`));
      }
      setNotice(`DNS record ${String(payload?.action || "updated")} successfully.`);
      await loadDnsRecords(activeZoneId);
    } catch (submitError) {
      setError(submitError instanceof Error ? submitError.message : "Failed to upsert DNS record.");
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    void refreshAll();
  });

  return (
    <div class="flex h-full min-h-0 flex-col bg-[#0c0d11]" data-testid="cloudflare-panel">
      <div class="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
        <div>
          <p class="text-xs uppercase tracking-wide text-neutral-400">Cloudflare Control</p>
          <p class="text-[11px] text-neutral-500">Domains, tunnels, DNS, and access apps via local bridge</p>
        </div>
        <div class="flex items-center gap-1.5">
          <select
            class="h-7 rounded border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200"
            value={zoneId()}
            onChange={(event) => {
              const nextZoneId = String(event.currentTarget.value || "").trim();
              setZoneId(nextZoneId);
              void loadDnsRecords(nextZoneId);
            }}
            data-testid="cloudflare-zone-select"
          >
            <option value="">Select domain</option>
            <For each={zones()}>
              {(zone) => <option value={zone.id}>{String(zone?.name || zone?.id || "zone")}</option>}
            </For>
          </select>
          <button
            type="button"
            class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)]"
            onClick={() => void refreshAll()}
            data-testid="cloudflare-refresh"
            disabled={loading()}
          >
            <FiRefreshCw size={12} />
            Refresh
          </button>
          <a
            class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)]"
            href="https://dash.cloudflare.com"
            target="_blank"
            rel="noopener noreferrer"
          >
            <FiExternalLink size={12} />
            Open Dashboard
          </a>
        </div>
      </div>

      <Show when={error()}>
        <div class="border-b border-red-800 bg-red-900/35 px-3 py-2 text-[11px] text-red-100" data-testid="cloudflare-panel-error">
          {error()}
        </div>
      </Show>
      <Show when={notice()}>
        <div class="border-b border-emerald-800 bg-emerald-900/25 px-3 py-2 text-[11px] text-emerald-100" data-testid="cloudflare-panel-notice">
          {notice()}
        </div>
      </Show>

      <div class="grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-auto p-3 lg:grid-cols-2 xl:grid-cols-3">
        <section class="rounded border border-neutral-800 bg-neutral-950/50 p-3" data-testid="cloudflare-zones-list">
          <p class="text-[11px] font-medium uppercase tracking-wide text-neutral-300">Domains (Zones)</p>
          <p class="mt-1 text-[10px] text-neutral-500">{zones().length} zones</p>
          <div class="mt-2 max-h-48 space-y-1 overflow-auto">
            <For each={zones()}>
              {(zone) => (
                <button
                  type="button"
                  class={`w-full rounded border px-2 py-1 text-left text-[10px] ${zoneId() === zone.id ? "border-orange-500 bg-orange-500/10 text-orange-100" : "border-neutral-800 bg-black/20 text-neutral-200 hover:border-neutral-700"}`}
                  onClick={() => {
                    const nextZoneId = String(zone?.id || "").trim();
                    setZoneId(nextZoneId);
                    void loadDnsRecords(nextZoneId);
                  }}
                >
                  {zoneLabel(zone)}
                </button>
              )}
            </For>
            <Show when={zones().length === 0 && !loading()}>
              <p class="text-[10px] text-neutral-500">No zones found.</p>
            </Show>
          </div>
        </section>

        <section class="rounded border border-neutral-800 bg-neutral-950/50 p-3" data-testid="cloudflare-tunnels-list">
          <p class="text-[11px] font-medium uppercase tracking-wide text-neutral-300">Tunnels</p>
          <p class="mt-1 text-[10px] text-neutral-500">{tunnels().length} tunnels {accountId() ? `on account ${accountId()}` : ""}</p>
          <div class="mt-2 max-h-48 space-y-1 overflow-auto">
            <For each={tunnels()}>
              {(tunnel) => (
                <div class="rounded border border-neutral-800 bg-black/20 px-2 py-1 text-[10px] text-neutral-200">
                  <p class="truncate">{String(tunnel?.name || tunnel?.id || "Tunnel")}</p>
                  <p class="truncate text-neutral-500">{String(tunnel?.status || "unknown")}</p>
                </div>
              )}
            </For>
            <Show when={tunnels().length === 0 && !loading()}>
              <p class="text-[10px] text-neutral-500">No tunnels found.</p>
            </Show>
          </div>
        </section>

        <section class="rounded border border-neutral-800 bg-neutral-950/50 p-3" data-testid="cloudflare-access-list">
          <p class="text-[11px] font-medium uppercase tracking-wide text-neutral-300">Access Apps</p>
          <p class="mt-1 text-[10px] text-neutral-500">{apps().length} apps</p>
          <div class="mt-2 max-h-52 space-y-1 overflow-auto">
            <For each={apps()}>
              {(app) => (
                <div class="rounded border border-neutral-800 bg-black/20 px-2 py-1 text-[10px] text-neutral-200">
                  <p class="truncate">{String(app?.name || app?.id || "Access App")}</p>
                  <p class="truncate text-neutral-500">{String(app?.domain || app?.aud || "")}</p>
                </div>
              )}
            </For>
            <Show when={apps().length === 0 && !loading()}>
              <p class="text-[10px] text-neutral-500">No access apps found.</p>
            </Show>
          </div>
        </section>

        <section class="rounded border border-neutral-800 bg-neutral-950/50 p-3" data-testid="cloudflare-workers-list">
          <p class="text-[11px] font-medium uppercase tracking-wide text-neutral-300">Workers</p>
          <p class="mt-1 text-[10px] text-neutral-500">{workers().length} scripts</p>
          <div class="mt-2 max-h-52 space-y-1 overflow-auto">
            <For each={workers()}>
              {(worker) => (
                <div class="rounded border border-neutral-800 bg-black/20 px-2 py-1 text-[10px] text-neutral-200">
                  <p class="truncate">{String(worker?.id || worker?.name || "Worker")}</p>
                  <p class="truncate text-neutral-500">{String(worker?.modified_on || worker?.created_on || "")}</p>
                </div>
              )}
            </For>
            <Show when={workers().length === 0 && !loading()}>
              <p class="text-[10px] text-neutral-500">No workers found.</p>
            </Show>
          </div>
        </section>

        <section class="rounded border border-neutral-800 bg-neutral-950/50 p-3" data-testid="cloudflare-pages-list">
          <p class="text-[11px] font-medium uppercase tracking-wide text-neutral-300">Pages</p>
          <p class="mt-1 text-[10px] text-neutral-500">{pages().length} projects</p>
          <div class="mt-2 max-h-52 space-y-1 overflow-auto">
            <For each={pages()}>
              {(page) => (
                <div class="rounded border border-neutral-800 bg-black/20 px-2 py-1 text-[10px] text-neutral-200">
                  <p class="truncate">{String(page?.name || page?.id || "Page Project")}</p>
                  <p class="truncate text-neutral-500">{String(page?.subdomain || page?.domains?.[0] || "")}</p>
                </div>
              )}
            </For>
            <Show when={pages().length === 0 && !loading()}>
              <p class="text-[10px] text-neutral-500">No pages projects found.</p>
            </Show>
          </div>
        </section>

        <section class="rounded border border-neutral-800 bg-neutral-950/50 p-3" data-testid="cloudflare-dns-records-list">
          <div class="flex items-center justify-between gap-2">
            <div>
              <p class="text-[11px] font-medium uppercase tracking-wide text-neutral-300">DNS</p>
              <p class="mt-1 text-[10px] text-neutral-500">{records().length} records</p>
            </div>
            <p class="text-[10px] text-neutral-500">{zoneId() ? `zone: ${zoneId()}` : "select domain above"}</p>
          </div>

          <div class="mt-2 rounded border border-neutral-800 bg-black/20 p-2 text-[10px] text-neutral-200">
            <p class="mb-1 text-neutral-400">Quick DNS upsert</p>
            <div class="grid grid-cols-2 gap-1">
              <select
                class="h-7 rounded border border-neutral-700 bg-neutral-900 px-2"
                value={form().type}
                onChange={(event) => setForm((prev) => ({ ...prev, type: event.currentTarget.value }))}
              >
                <option value="A">A</option>
                <option value="AAAA">AAAA</option>
                <option value="CNAME">CNAME</option>
                <option value="TXT">TXT</option>
              </select>
              <input
                class="h-7 rounded border border-neutral-700 bg-neutral-900 px-2"
                placeholder="Name (e.g. app.example.com)"
                value={form().name}
                onInput={(event) => setForm((prev) => ({ ...prev, name: event.currentTarget.value }))}
              />
              <input
                class="col-span-2 h-7 rounded border border-neutral-700 bg-neutral-900 px-2"
                placeholder="Content (IP, hostname, or TXT value)"
                value={form().content}
                onInput={(event) => setForm((prev) => ({ ...prev, content: event.currentTarget.value }))}
              />
              <input
                class="h-7 rounded border border-neutral-700 bg-neutral-900 px-2"
                placeholder="TTL (1=auto)"
                value={form().ttl}
                onInput={(event) => setForm((prev) => ({ ...prev, ttl: event.currentTarget.value }))}
              />
              <label class="inline-flex h-7 items-center gap-1 rounded border border-neutral-700 bg-neutral-900 px-2 text-neutral-300">
                <input
                  type="checkbox"
                  checked={form().proxied}
                  onChange={(event) => setForm((prev) => ({ ...prev, proxied: event.currentTarget.checked }))}
                />
                Proxied
              </label>
            </div>
            <button
              type="button"
              class="mt-2 inline-flex h-7 items-center gap-1 rounded border border-orange-700/60 bg-orange-700/20 px-2 text-[10px] text-orange-100 hover:border-orange-500"
              onClick={() => void submitDnsUpsert()}
              data-testid="cloudflare-dns-upsert-submit"
              disabled={loading() || !zoneId()}
            >
              <FiSave size={12} />
              Save DNS Record
            </button>
          </div>

          <div class="mt-2 max-h-48 space-y-1 overflow-auto">
            <For each={records()}>
              {(record) => (
                <div class="rounded border border-neutral-800 bg-black/20 px-2 py-1 text-[10px] text-neutral-200">
                  <p class="truncate">
                    {String(record?.type || "?")} {String(record?.name || "")}
                  </p>
                  <p class="truncate text-neutral-500">{String(record?.content || "")}</p>
                </div>
              )}
            </For>
            <Show when={records().length === 0 && !loading()}>
              <p class="text-[10px] text-neutral-500">No DNS records found for selected zone.</p>
            </Show>
          </div>
        </section>
      </div>
    </div>
  );
}
