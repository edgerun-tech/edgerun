const TOKEN_KEYS: Record<string, string> = {
  cloudflare: 'cloudflare_token',
  vercel: 'vercel_token',
  hetzner: 'hetzner_token',
  github: 'github_token',
  google: 'google_token',
  qwen: 'qwen_token',
};

export function getToken(provider: string): string | null {
  if (typeof window === 'undefined') return null;
  const key = TOKEN_KEYS[provider] || `${provider}_token`;
  const raw = localStorage.getItem(key);
  if (!raw) return null;

  if (provider === 'qwen') {
    try {
      const parsed = JSON.parse(raw);
      return parsed?.access_token || null;
    } catch {
      return null;
    }
  }

  return raw;
}

export function setToken(provider: string, token: string): void {
  if (typeof window === 'undefined') return;
  const key = TOKEN_KEYS[provider] || `${provider}_token`;
  localStorage.setItem(key, token);
}

export function clearToken(provider: string): void {
  if (typeof window === 'undefined') return;
  const key = TOKEN_KEYS[provider] || `${provider}_token`;
  localStorage.removeItem(key);
}
