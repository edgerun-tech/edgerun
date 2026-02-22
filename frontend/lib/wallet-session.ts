// SPDX-License-Identifier: Apache-2.0
export const WALLET_SESSION_EVENT = 'edgerun:wallet-session'

const STORAGE_KEY = 'edgerun.wallet.session.v1'

export type WalletSessionState = {
  connected: boolean
  address: string
  provider: string
}

const DEFAULT_STATE: WalletSessionState = {
  connected: false,
  address: '',
  provider: ''
}

export function readWalletSession(): WalletSessionState {
  if (typeof window === 'undefined') return DEFAULT_STATE
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return DEFAULT_STATE
    const parsed = JSON.parse(raw) as Partial<WalletSessionState>
    const connected = Boolean(parsed.connected) && typeof parsed.address === 'string' && parsed.address.trim().length > 0
    return {
      connected,
      address: connected ? String(parsed.address || '') : '',
      provider: connected ? String(parsed.provider || '') : ''
    }
  } catch {
    return DEFAULT_STATE
  }
}

export function writeWalletSession(next: WalletSessionState): WalletSessionState {
  const normalized: WalletSessionState = {
    connected: Boolean(next.connected) && next.address.trim().length > 0,
    address: next.connected ? next.address.trim() : '',
    provider: next.connected ? next.provider.trim() : ''
  }
  if (typeof window !== 'undefined') {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized))
    } catch {
      // ignore local storage errors
    }
    window.dispatchEvent(new CustomEvent(WALLET_SESSION_EVENT, { detail: normalized }))
  }
  return normalized
}

export function clearWalletSession(): WalletSessionState {
  return writeWalletSession(DEFAULT_STATE)
}
