import type { WritableAtom } from "nanostores"

export function patchStore<T extends Record<string, unknown>>(
  store: WritableAtom<T>,
  patch: Partial<T>,
): void {
  store.set({ ...store.get(), ...patch })
}
