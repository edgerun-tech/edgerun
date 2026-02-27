const EMAILS_KEY = "demo-emails";
const SETTINGS_KEY = "demo-email-settings";
const EMAIL_INDEX_KEY = "demo-email-index-v1";
const MAX_PERSISTED_EMAILS = 400;
const MIN_PERSISTED_EMAILS = 50;

/**
 * @typedef {object} StoredEmail
 * @property {string} id
 * @property {string} threadId
 * @property {string} subject
 * @property {string} from
 * @property {string} to
 * @property {string} date
 * @property {string} snippet
 * @property {string[]=} labelIds
 * @property {string=} searchText
 */

/**
 * @typedef {object} SyncSettings
 * @property {number} historyDays
 * @property {number} maxEmails
 * @property {number=} lastSync
 */

const isDemoSeedEmail = (email) => {
  if (!email || typeof email !== "object") return false;
  const id = String(email.id || "");
  const from = String(email.from || "").toLowerCase();
  return id === "seed-1" || from.includes("demo@local.dev");
};
const normalizeText = (value) =>
  String(value || "")
    .toLowerCase()
    .replace(/<[^>]+>/g, " ")
    .replace(/[^a-z0-9\s@._-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
const buildSearchText = (email) =>
  normalizeText([
    email.subject,
    email.from,
    email.to,
    email.snippet,
    Array.isArray(email.labelIds) ? email.labelIds.join(" ") : ""
  ].join(" "));
const clampText = (value, max) => String(value || "").slice(0, max);
const normalizeEmailDate = (value) => {
  if (!value) return new Date(0).toISOString();
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? new Date(0).toISOString() : parsed.toISOString();
};
const toStoredEmail = (email) => ({
  id: String(email?.id || ""),
  threadId: String(email?.threadId || ""),
  subject: clampText(email?.subject || "(No Subject)", 300),
  from: clampText(email?.from || "", 300),
  to: clampText(email?.to || "", 500),
  date: normalizeEmailDate(email?.date),
  snippet: clampText(email?.snippet || "", 900),
  labelIds: Array.isArray(email?.labelIds) ? email.labelIds.map((label) => String(label)).slice(0, 24) : []
});
const uniqueTokens = (text) => Array.from(new Set(
  normalizeText(text)
    .split(" ")
    .filter((token) => token.length >= 2)
));
const isQuotaExceededError = (error) =>
  !!error && typeof error === "object" && (
    error.name === "QuotaExceededError"
    || error.code === 22
    || error.code === 1014
    || String(error.message || "").toLowerCase().includes("quota")
  );
/** @returns {StoredEmail[]} */
function readEmails() {
  if (typeof window === "undefined") return [];
  const raw = localStorage.getItem(EMAILS_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item) => !isDemoSeedEmail(item));
  } catch {
    return [];
  }
}
function readEmailIndex() {
  if (typeof window === "undefined") return {};
  const raw = localStorage.getItem(EMAIL_INDEX_KEY);
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}
function writeEmailIndex(index) {
  if (typeof window === "undefined") return;
  localStorage.setItem(EMAIL_INDEX_KEY, JSON.stringify(index));
}
function buildEmailIndex(emails) {
  const next = {};
  for (const email of emails) {
    const id = String(email.id || "");
    if (!id) continue;
    const tokens = uniqueTokens(email.searchText || "");
    for (const token of tokens) {
      if (!next[token]) next[token] = [];
      next[token].push(id);
    }
  }
  return next;
}
/** @param {StoredEmail[]} emails */
function writeEmails(emails) {
  if (typeof window === "undefined") return;
  const prepared = emails
    .map((email) => toStoredEmail(email))
    .filter((email) => email.id)
    .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
    .slice(0, MAX_PERSISTED_EMAILS)
    .map((email) => ({ ...email, searchText: buildSearchText(email) }));
  let keep = prepared.length;
  while (keep >= MIN_PERSISTED_EMAILS) {
    const slice = prepared.slice(0, keep);
    try {
      localStorage.setItem(EMAILS_KEY, JSON.stringify(slice));
      writeEmailIndex(buildEmailIndex(slice));
      return;
    } catch (error) {
      if (!isQuotaExceededError(error)) throw error;
      keep = Math.floor(keep * 0.7);
    }
  }
  localStorage.removeItem(EMAILS_KEY);
  localStorage.removeItem(EMAIL_INDEX_KEY);
  throw new Error("Email cache quota exceeded. Reduce sync scope in Gmail settings.");
}
/** @returns {SyncSettings} */
function readSettings() {
  if (typeof window === "undefined") return { historyDays: 30, maxEmails: 500 };
  const raw = localStorage.getItem(SETTINGS_KEY);
  if (!raw) return { historyDays: 30, maxEmails: 500 };
  try {
    return JSON.parse(raw);
  } catch {
    return { historyDays: 30, maxEmails: 500 };
  }
}
async function getSyncSettings() {
  return readSettings();
}
/** @param {SyncSettings} settings */
async function saveSyncSettings(settings) {
  if (typeof window !== "undefined") {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  }
}
async function getEmailCount() {
  return readEmails().length;
}
async function getAllEmails() {
  const emails = readEmails();
  let changed = false;
  const normalized = emails.map((email) => {
    if (email.searchText) return email;
    changed = true;
    return { ...email, searchText: buildSearchText(email) };
  });
  if (changed) {
    writeEmails(normalized);
  }
  return normalized;
}
async function getEmailIdSet() {
  return new Set(readEmails().map((email) => String(email.id || "")).filter(Boolean));
}
/** @param {string} id */
async function getEmail(id) {
  return readEmails().find((e) => e.id === id) ?? null;
}
/** @param {StoredEmail[]} emails */
async function saveEmails(emails) {
  const existing = readEmails();
  const mergedById = new Map();
  for (const email of existing) {
    const id = String(email?.id || "");
    if (!id) continue;
    mergedById.set(id, email);
  }
  for (const email of emails || []) {
    const id = String(email?.id || "");
    if (!id) continue;
    const prev = mergedById.get(id) || {};
    mergedById.set(id, { ...prev, ...email, id });
  }
  writeEmails(Array.from(mergedById.values()));
}
/** @param {number} historyDays */
async function deleteOldEmails(historyDays) {
  const cutoff = Date.now() - historyDays * 24 * 60 * 60 * 1e3;
  const next = readEmails().filter((e) => new Date(e.date).getTime() >= cutoff);
  writeEmails(next);
}
/**
 * @param {string} query
 * @param {number} [limit]
 * @returns {Promise<StoredEmail[]>}
 */
async function searchEmails(query, limit = 250) {
  const rawQuery = normalizeText(query);
  if (!rawQuery) return getAllEmails();
  const emails = await getAllEmails();
  const byId = new Map(emails.map((email) => [email.id, email]));
  const tokens = uniqueTokens(rawQuery);
  const index = readEmailIndex();
  let candidateIds = null;
  for (const token of tokens) {
    const bucket = Array.isArray(index[token]) ? index[token] : [];
    const bucketSet = new Set(bucket);
    if (!candidateIds) {
      candidateIds = bucketSet;
      continue;
    }
    candidateIds = new Set(Array.from(candidateIds).filter((id) => bucketSet.has(id)));
  }
  const candidates = candidateIds
    ? Array.from(candidateIds).map((id) => byId.get(id)).filter(Boolean)
    : emails;
  const scored = candidates.map((email) => {
    const haystack = email.searchText || buildSearchText(email);
    let score = 0;
    for (const token of tokens) {
      if (haystack.startsWith(token)) score += 30;
      if (haystack.includes(token)) score += 15;
      if (normalizeText(email.subject).includes(token)) score += 20;
      if (normalizeText(email.from).includes(token)) score += 10;
    }
    return { email, score };
  }).filter((entry) => entry.score > 0).sort((a, b) =>
    b.score - a.score || new Date(b.email.date).getTime() - new Date(a.email.date).getTime()
  );
  return scored.slice(0, Math.max(1, limit)).map((entry) => entry.email);
}
async function updateLastSync() {
  const settings = readSettings();
  settings.lastSync = Date.now();
  if (typeof window !== "undefined") {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  }
}
export {
  deleteOldEmails,
  getAllEmails,
  getEmailIdSet,
  getEmail,
  getEmailCount,
  getSyncSettings,
  searchEmails,
  saveEmails,
  saveSyncSettings,
  updateLastSync
};
