export type StoredEmail = {
  id: string;
  threadId: string;
  subject: string;
  from: string;
  to: string;
  date: string;
  snippet: string;
  body?: string;
  html?: string;
  labelIds?: string[];
};

type SyncSettings = {
  historyDays: number;
  maxEmails: number;
  lastSync?: number;
};

const EMAIL_KEY = 'cloud-os-email-cache-v1';
const SETTINGS_KEY = 'cloud-os-email-sync-settings-v1';

function readEmails(): StoredEmail[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(EMAIL_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function writeEmails(emails: StoredEmail[]): void {
  if (typeof window === 'undefined') return;
  localStorage.setItem(EMAIL_KEY, JSON.stringify(emails));
}

function readSettings(): SyncSettings {
  if (typeof window === 'undefined') return { historyDays: 30, maxEmails: 500 };
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { historyDays: 30, maxEmails: 500 };
    const parsed = JSON.parse(raw);
    return {
      historyDays: Number(parsed.historyDays) || 30,
      maxEmails: Number(parsed.maxEmails) || 500,
      lastSync: typeof parsed.lastSync === 'number' ? parsed.lastSync : undefined,
    };
  } catch {
    return { historyDays: 30, maxEmails: 500 };
  }
}

function writeSettings(settings: SyncSettings): void {
  if (typeof window === 'undefined') return;
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

export async function getAllEmails(): Promise<StoredEmail[]> {
  return readEmails();
}

export async function getEmail(id: string): Promise<StoredEmail | null> {
  return readEmails().find((email) => email.id === id) || null;
}

export async function saveEmails(emails: StoredEmail[]): Promise<void> {
  const current = readEmails();
  const merged = [...emails, ...current.filter((existing) => !emails.some((item) => item.id === existing.id))];
  merged.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());
  writeEmails(merged);
}

export async function getEmailCount(): Promise<number> {
  return readEmails().length;
}

export async function deleteOldEmails(historyDays: number): Promise<void> {
  if (historyDays <= 0) return;
  const cutoff = Date.now() - historyDays * 24 * 60 * 60 * 1000;
  const filtered = readEmails().filter((email) => {
    const ts = new Date(email.date).getTime();
    return Number.isFinite(ts) && ts >= cutoff;
  });
  writeEmails(filtered);
}

export async function saveSyncSettings(settings: { historyDays: number; maxEmails: number }): Promise<void> {
  const current = readSettings();
  writeSettings({ ...current, ...settings });
}

export async function getSyncSettings(): Promise<SyncSettings> {
  return readSettings();
}

export async function updateLastSync(): Promise<void> {
  const current = readSettings();
  writeSettings({ ...current, lastSync: Date.now() });
}
