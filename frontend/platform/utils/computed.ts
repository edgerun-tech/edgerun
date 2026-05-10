import { computed, ReadableAtom } from "nanostores"

export function computedMapToList<TStore, TVal>(
  store: ReadableAtom<TStore>,
  getMap: (state: TStore) => Map<string, TVal>,
): ReadableAtom<TVal[]> {
  return computed(store, (s) => Array.from(getMap(s).values()))
}
