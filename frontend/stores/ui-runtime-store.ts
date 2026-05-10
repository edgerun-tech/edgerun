import { atom } from "nanostores"

export const clientMountedStore = atom(false)

export function markClientMounted() {
  if (!clientMountedStore.get()) clientMountedStore.set(true)
}
