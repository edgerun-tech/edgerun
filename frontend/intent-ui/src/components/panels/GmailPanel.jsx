import { TbOutlineArrowLeft, TbOutlineRefresh, TbOutlineMail, TbOutlineSettings, TbOutlineCheck, TbOutlineSearch } from "solid-icons/tb";
import { createSignal, For, Show, onMount, onCleanup, createEffect, createMemo } from "solid-js";
import VirtualAnimatedList from "../common/VirtualAnimatedList";
import { openWorkflowIntegrations } from "../../stores/workflow-ui";
const GOOGLE_TOKEN_KEY = "google_token";
const GOOGLE_REFRESH_KEY = "google_refresh";
const HISTORY_OPTIONS = [
  { value: 7, label: "7 days" },
  { value: 30, label: "30 days" },
  { value: 90, label: "90 days" },
  { value: 365, label: "1 year" },
  { value: -1, label: "All time" }
];
const EMAIL_LIMIT_OPTIONS = [
  { value: 100, label: "100 emails" },
  { value: 250, label: "250 emails" },
  { value: 500, label: "500 emails" },
  { value: 1e3, label: "1,000 emails" }
];
const AUTO_SYNC_INTERVAL_MS = 10 * 60 * 1e3;
function GmailPanel() {
  const [emails, setEmails] = createSignal([]);
  const [selectedEmail, setSelectedEmail] = createSignal(null);
  const [loading, setLoading] = createSignal(false);
  const [syncing, setSyncing] = createSignal(false);
  const [error, setError] = createSignal(null);
  const [showSettings, setShowSettings] = createSignal(false);
  const [settings, setSettings] = createSignal({ historyDays: 30, maxEmails: 500 });
  const [lastSync, setLastSync] = createSignal(null);
  const [emailCount, setEmailCount] = createSignal(0);
  const [syncProgress, setSyncProgress] = createSignal("");
  const [searchQuery, setSearchQuery] = createSignal("");
  const [searching, setSearching] = createSignal(false);
  const [filteredEmails, setFilteredEmails] = createSignal([]);
  const [googleToken, setGoogleToken] = createSignal("");
  const [googleRefresh, setGoogleRefresh] = createSignal("");
  let authPoll = null;
  let onStorageHandler = null;
  let onFocusHandler = null;
  let listRef;
  const visibleEmails = createMemo(() => searchQuery().trim() ? filteredEmails() : emails());
  const token = () => googleToken();
  const refreshToken = () => googleRefresh();
  const refreshAuthState = () => {
    setGoogleToken(localStorage.getItem(GOOGLE_TOKEN_KEY) || "");
    setGoogleRefresh(localStorage.getItem(GOOGLE_REFRESH_KEY) || "");
  };
  const loadSettings = async () => {
    const { getSyncSettings, getEmailCount, getAllEmails } = await import("../lib/db");
    const saved = await getSyncSettings();
    if (saved) {
      setSettings({ historyDays: saved.historyDays, maxEmails: saved.maxEmails });
      setLastSync(saved.lastSync || null);
    }
    const count = await getEmailCount();
    setEmailCount(count);
    const localEmails = await getAllEmails();
    if (localEmails.length > 0) {
      setEmails(localEmails.map((e) => ({
        id: e.id,
        threadId: e.threadId,
        subject: e.subject,
        from: e.from,
        snippet: e.snippet,
        date: e.date,
        labelIds: e.labelIds,
        searchText: e.searchText
      })).sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime()));
    }
  };
  const runSearch = async (query) => {
    const trimmed = query.trim();
    if (!trimmed) {
      setFilteredEmails([]);
      return;
    }
    setSearching(true);
    try {
      const { searchEmails } = await import("../lib/db");
      const matches = await searchEmails(trimmed, 500);
      setFilteredEmails(matches.map((e) => ({
        id: e.id,
        threadId: e.threadId,
        subject: e.subject,
        from: e.from,
        snippet: e.snippet,
        date: e.date,
        labelIds: e.labelIds,
        searchText: e.searchText
      })));
    } finally {
      setSearching(false);
    }
  };
  const saveSettings = async () => {
    const { saveSyncSettings } = await import("../lib/db");
    await saveSyncSettings(settings());
    setShowSettings(false);
  };
  const readResponseError = async (response, fallback) => {
    try {
      const payload = await response.json();
      if (typeof payload?.error === "string" && payload.error.trim()) return payload.error;
      if (typeof payload?.message === "string" && payload.message.trim()) return payload.message;
    } catch {
    }
    return fallback;
  };
  const syncEmails = async ({ force = false } = {}) => {
    if (!token()) {
      setError("Not connected to Gmail");
      return;
    }
    setSyncing(true);
    setError(null);
    setSyncProgress("Starting sync...");
    try {
      const {
        saveEmails,
        getEmailCount,
        deleteOldEmails,
        updateLastSync,
        getSyncSettings,
        getEmailIdSet
      } = await import("../lib/db");
      const historyDays = settings().historyDays;
      const maxEmails = settings().maxEmails;
      const persistedSettings = await getSyncSettings();
      const lastSyncedAt = Number(persistedSettings?.lastSync || 0);
      const elapsed = Date.now() - lastSyncedAt;
      if (!force && lastSyncedAt > 0 && elapsed < AUTO_SYNC_INTERVAL_MS) {
        const waitMinutes = Math.max(1, Math.ceil((AUTO_SYNC_INTERVAL_MS - elapsed) / 6e4));
        setLastSync(lastSyncedAt);
        setSyncProgress(`Next auto sync in ${waitMinutes}m...`);
        setTimeout(() => setSyncProgress(""), 2e3);
        return;
      }
      let query = `/api/google/messages?token=${encodeURIComponent(token())}&maxResults=${maxEmails}`;
      if (historyDays > 0) {
        const cutoff = /* @__PURE__ */ new Date();
        cutoff.setDate(cutoff.getDate() - historyDays);
        query += `&after=${Math.floor(cutoff.getTime() / 1e3)}`;
      }
      setSyncProgress("Fetching email list...");
      const res = await fetch(query);
      if (!res.ok) {
        if (res.status === 401) {
          await refreshAccessToken();
          return;
        }
        throw new Error(await readResponseError(res, "Failed to fetch emails"));
      }
      const data = await res.json();
      const messages = data.messages || [];
      const knownEmailIds = await getEmailIdSet();
      const unknownMessages = messages.filter((message) => {
        const id = String(message?.id || "");
        return id && !knownEmailIds.has(id);
      });
      if (unknownMessages.length === 0) {
        await updateLastSync();
        setLastSync(Date.now());
        setSyncProgress("Mailbox already up to date.");
        setTimeout(() => setSyncProgress(""), 2e3);
        return;
      }
      setSyncProgress(`Syncing ${unknownMessages.length} new emails...`);
      const detailedEmails = [];
      const batchSize = 5;
      for (let i = 0; i < unknownMessages.length; i += batchSize) {
        const batch = unknownMessages.slice(i, i + batchSize);
        const details = await Promise.all(
          batch.map((msg) => fetchEmailDetail(msg.id))
        );
        const validDetails = details.filter((d) => d !== null);
        detailedEmails.push(...validDetails);
        setSyncProgress(`Synced ${Math.min(i + batchSize, unknownMessages.length)}/${unknownMessages.length} new emails...`);
      }
      await saveEmails(detailedEmails);
      if (historyDays > 0) {
        setSyncProgress("Cleaning old emails...");
        await deleteOldEmails(historyDays);
      }
      await updateLastSync();
      const count = await getEmailCount();
      setEmailCount(count);
      setLastSync(Date.now());
      await loadSettings();
      setSyncProgress("Sync complete!");
      setTimeout(() => setSyncProgress(""), 2e3);
    } catch (e) {
      console.error("Sync failed:", e);
      setError(e instanceof Error ? e.message : "Failed to sync emails");
    } finally {
      setSyncing(false);
    }
  };
  const fetchEmailDetail = async (id) => {
    if (!token()) return null;
    try {
      const res = await fetch(`/api/google/message/${id}?token=${encodeURIComponent(token())}`);
      if (res.ok) {
        const data = await res.json();
        return {
          id: data.id,
          threadId: data.threadId,
          subject: data.subject || "(No Subject)",
          from: data.from,
          to: data.to,
          date: data.date,
          snippet: data.snippet || "",
          body: data.body,
          html: data.html,
          labelIds: data.labelIds
        };
      }
    } catch (e) {
      console.error("Failed to fetch email detail:", e);
    }
    return null;
  };
  const loadEmailDetail = async (id) => {
    const { getEmail } = await import("../lib/db");
    const cached = await getEmail(id);
    if (cached) {
      setSelectedEmail({
        id: cached.id,
        subject: cached.subject,
        from: cached.from,
        to: cached.to,
        date: cached.date,
        snippet: cached.snippet,
        body: cached.body,
        html: cached.html
      });
      if (cached.body || cached.html || !token()) return;
    }
    if (!token()) return;
    setLoading(true);
    try {
      const res = await fetch(`/api/google/message/${id}?token=${encodeURIComponent(token())}`);
      if (res.ok) {
        const data = await res.json();
        setSelectedEmail(data);
      } else {
        setError(await readResponseError(res, "Failed to load email"));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load email");
    }
    setLoading(false);
  };
  const refreshAccessToken = async () => {
    const refresh = refreshToken();
    if (!refresh) {
      setError("Please reconnect to Gmail");
      return;
    }
    try {
      const res = await fetch("/api/google/refresh", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ refresh_token: refresh })
      });
      if (res.ok) {
        const data = await res.json();
        localStorage.setItem(GOOGLE_TOKEN_KEY, data.access_token);
        await syncEmails({ force: true });
      } else {
        setError("Please reconnect to Gmail");
      }
    } catch (e) {
      setError("Please reconnect to Gmail");
    }
  };
  const formatDate = (dateStr) => {
    if (!dateStr) return "";
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString();
    } catch {
      return dateStr;
    }
  };
  const formatLastSync = (timestamp) => {
    if (!timestamp) return "Never";
    const date = new Date(timestamp);
    return date.toLocaleString();
  };
  const goBack = () => {
    setSelectedEmail(null);
  };
  onMount(() => {
    refreshAuthState();
    onStorageHandler = (event) => {
      if (event.key && event.key !== GOOGLE_TOKEN_KEY && event.key !== GOOGLE_REFRESH_KEY) return;
      refreshAuthState();
    };
    onFocusHandler = () => {
      refreshAuthState();
    };
    window.addEventListener("storage", onStorageHandler);
    window.addEventListener("focus", onFocusHandler);
    authPoll = window.setInterval(refreshAuthState, 3000);
    if (token()) {
      loadSettings();
    }
  });
  onCleanup(() => {
    if (onStorageHandler) {
      window.removeEventListener("storage", onStorageHandler);
    }
    if (onFocusHandler) {
      window.removeEventListener("focus", onFocusHandler);
    }
    if (authPoll) {
      window.clearInterval(authPoll);
    }
  });
  createEffect(() => {
    if (token() && emailCount() === 0 && !syncing()) {
      syncEmails({ force: false });
    }
  });
  return <div class="h-full flex flex-col bg-[#1a1a1a] text-neutral-200 p-4">
      <Show when={!token()}>
        <div class="flex-1 flex items-center justify-center text-neutral-400">
          <div class="text-center">
            <p>Please connect to Gmail first</p>
            <button
              type="button"
              class="mt-3 rounded-md border border-blue-500/40 bg-blue-600/15 px-3 py-1.5 text-xs text-blue-200 hover:bg-blue-600/25"
              onClick={() => openWorkflowIntegrations("google")}
            >
              Connect Google
            </button>
          </div>
        </div>
      </Show>

      <Show when={token()}>
        <Show when={selectedEmail()}>
          <div class="p-3 border-b border-neutral-800 flex items-center gap-2">
            <button
    type="button"
    onClick={goBack}
    class="p-1.5 rounded hover:bg-neutral-700 transition-colors"
  >
              <TbOutlineArrowLeft size={18} />
            </button>
            <span class="font-medium truncate">{selectedEmail()?.subject}</span>
          </div>
          
          <div class="flex-1 overflow-auto p-4">
            <div class="space-y-4">
              <div>
                <div class="text-sm text-neutral-400">From</div>
                <div class="text-neutral-200">{selectedEmail()?.from}</div>
              </div>
              <div>
                <div class="text-sm text-neutral-400">To</div>
                <div class="text-neutral-200">{selectedEmail()?.to}</div>
              </div>
              <div>
                <div class="text-sm text-neutral-400">Date</div>
                <div class="text-neutral-200">{selectedEmail()?.date}</div>
              </div>
              <div class="pt-4 border-t border-neutral-800">
                <Show when={selectedEmail()?.html} fallback={<div class="text-neutral-200 whitespace-pre-wrap">{selectedEmail()?.body || selectedEmail()?.snippet}</div>}>
                  <div class="text-neutral-200" innerHTML={selectedEmail()?.html} />
                </Show>
              </div>
            </div>
          </div>
        </Show>

        <Show when={!selectedEmail()}>
          <div class="p-3 border-b border-neutral-800 flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class="font-medium">Inbox</span>
              <Show when={emailCount() > 0}>
                <span class="text-xs text-neutral-500">({emailCount()} emails)</span>
              </Show>
              <Show when={searchQuery().trim()}>
                <span class="text-xs text-neutral-500">· {visibleEmails().length} match</span>
              </Show>
            </div>
            <div class="flex items-center gap-1">
              <button
    type="button"
    onClick={() => setShowSettings(true)}
    class="p-1.5 rounded hover:bg-neutral-700 transition-colors"
    title="Sync Settings"
  >
                <TbOutlineSettings size={16} />
              </button>
              <button
    type="button"
    onClick={() => syncEmails({ force: true })}
    disabled={syncing()}
    class="p-1.5 rounded hover:bg-neutral-700 transition-colors disabled:opacity-50"
    title="Sync"
  >
                <TbOutlineRefresh size={16} class={syncing() ? "animate-spin" : ""} />
              </button>
            </div>
          </div>
          <div class="border-b border-neutral-800 px-3 py-2">
            <div class="flex items-center gap-2 rounded-lg border border-neutral-700 bg-neutral-900/70 px-2 py-1.5">
              <TbOutlineSearch size={14} class="text-neutral-500" />
              <input
                type="text"
                value={searchQuery()}
                onInput={(event) => {
                  const value = event.currentTarget.value;
                  setSearchQuery(value);
                  void runSearch(value);
                }}
                placeholder="Search indexed emails..."
                class="w-full border-none bg-transparent text-sm text-neutral-200 outline-none placeholder:text-neutral-500"
              />
              <Show when={searching()}>
                <div class="h-3.5 w-3.5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
              </Show>
            </div>
          </div>

          <Show when={syncing() || syncProgress()}>
            <div class="px-3 py-2 bg-blue-900/30 border-b border-neutral-800">
              <div class="flex items-center gap-2 text-sm text-blue-300">
                <div class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
                {syncProgress()}
              </div>
            </div>
          </Show>

          <Show when={error()}>
            <div class="px-3 py-2 bg-red-900/30 border-b border-neutral-800">
              <p class="text-sm text-red-300">{error()}</p>
            </div>
          </Show>

          <Show when={lastSync()}>
            <div class="px-3 py-1.5 bg-neutral-800/50 border-b border-neutral-800">
              <p class="text-xs text-neutral-500">
                Last synced: {formatLastSync(lastSync())}
              </p>
            </div>
          </Show>

          <div class="flex-1 overflow-auto" ref={listRef}>
            <Show when={loading()}>
              <div class="flex items-center justify-center p-4">
                <div class="w-6 h-6 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />
              </div>
            </Show>

            <Show when={!loading() && visibleEmails().length > 0}>
              <VirtualAnimatedList
                items={visibleEmails}
                estimateSize={88}
                overscan={5}
                containerRef={() => listRef}
                layout="absolute"
                class="divide-y divide-neutral-800 relative"
                rowClass="absolute w-full"
                renderItem={(email) => (
                  <button
                    type="button"
                    onClick={() => loadEmailDetail(email.id)}
                    class="w-full p-3 hover:bg-neutral-800 transition-colors text-left"
                  >
                    <div class="flex items-center justify-between mb-1">
                      <span class="font-medium text-sm truncate">{email.subject}</span>
                      <span class="text-xs text-neutral-500">{formatDate(email.date)}</span>
                    </div>
                    <div class="text-xs text-neutral-400 truncate">{email.from}</div>
                    <div class="text-xs text-neutral-500 truncate mt-1">{email.snippet}</div>
                  </button>
                )}
              />
            </Show>
            
            <Show when={visibleEmails().length === 0 && !loading()}>
              <div class="p-8 text-center text-neutral-500">
                <TbOutlineMail size={32} class="mx-auto mb-2 opacity-50" />
                <Show
                  when={searchQuery().trim()}
                  fallback={<>
                    <p>No emails yet</p>
                    <button
    type="button"
    onClick={() => syncEmails({ force: true })}
    class="mt-2 text-blue-400 hover:text-blue-300 text-sm"
  >
                      Click to sync
                    </button>
                  </>}
                >
                  <p>No matching emails</p>
                </Show>
              </div>
            </Show>
          </div>
        </Show>
      </Show>

      <Show when={showSettings()}>
        <div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
          <div class="bg-neutral-800 rounded-2xl p-6 max-w-md w-full mx-4 border border-neutral-700">
            <h3 class="text-lg font-semibold mb-4">Sync Settings</h3>
            
            <div class="space-y-4">
              <div>
                <label for="historyDays" class="block text-sm text-neutral-400 mb-2">History to download</label>
                <select
    id="historyDays"
    value={settings().historyDays}
    onChange={(e) => setSettings((s) => ({ ...s, historyDays: parseInt(e.currentTarget.value) }))}
    class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-white text-sm focus:outline-none focus:border-blue-500"
  >
                  <For each={HISTORY_OPTIONS}>
                    {(opt) => <option value={opt.value}>{opt.label}</option>}
                  </For>
                </select>
              </div>
              
              <div>
                <label for="maxEmails" class="block text-sm text-neutral-400 mb-2">Maximum emails to keep</label>
                <select
    id="maxEmails"
    value={settings().maxEmails}
    onChange={(e) => setSettings((s) => ({ ...s, maxEmails: parseInt(e.currentTarget.value) }))}
    class="w-full px-3 py-2 bg-neutral-900 border border-neutral-700 rounded-lg text-white text-sm focus:outline-none focus:border-blue-500"
  >
                  <For each={EMAIL_LIMIT_OPTIONS}>
                    {(opt) => <option value={opt.value}>{opt.label}</option>}
                  </For>
                </select>
              </div>

              <div class="pt-4 border-t border-neutral-700">
                <div class="flex items-center justify-between text-sm">
                  <span class="text-neutral-400">Currently stored:</span>
                  <span class="text-white">{emailCount()} emails</span>
                </div>
              </div>
            </div>

            <div class="flex gap-3 mt-6">
              <button
    type="button"
    onClick={() => setShowSettings(false)}
    class="flex-1 px-4 py-2 bg-neutral-700 hover:bg-neutral-600 text-white rounded-lg transition-colors"
  >
                Cancel
              </button>
              <button
    type="button"
    onClick={saveSettings}
    class="flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors flex items-center justify-center gap-2"
  >
                <TbOutlineCheck size={18} />
                Save
              </button>
            </div>
          </div>
        </div>
      </Show>
    </div>;
}
export {
  GmailPanel as default
};
