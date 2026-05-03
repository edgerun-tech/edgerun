import { atom, computed } from "nanostores"
import type { ToastActionElement, ToastProps } from "@/components/ui/toast"

const TOAST_LIMIT = 1
const TOAST_REMOVE_DELAY = 1000000

type ToasterToast = ToastProps & {
  id: string
  title?: string | React.ReactNode
  description?: string | React.ReactNode
  action?: ToastActionElement
}

let count = 0
function genId() {
  count = (count + 1) % Number.MAX_SAFE_INTEGER
  return count.toString()
}

const toastTimeouts = new Map<string, ReturnType<typeof setTimeout>>()

function addToRemoveQueue(toastId: string) {
  if (toastTimeouts.has(toastId)) return
  const timeout = setTimeout(() => {
    toastTimeouts.delete(toastId)
    removeToast(toastId)
  }, TOAST_REMOVE_DELAY)
  toastTimeouts.set(toastId, timeout)
}

export const toastsStore = atom<ToasterToast[]>([])

export const hasOpenToastStore = computed(toastsStore, (toasts) => toasts.some((t) => t.open !== false))

export function addToast(toast: Omit<ToasterToast, "id">) {
  const id = genId()
  const newToast: ToasterToast = {
    ...toast,
    id,
    open: true,
    onOpenChange: (open) => {
      if (!open) dismissToast(id)
    },
  }
  toastsStore.set([newToast, ...toastsStore.get()].slice(0, TOAST_LIMIT))
  return { id, dismiss: () => dismissToast(id), update: (props: Partial<ToasterToast>) => updateToast(id, props) }
}

export function updateToast(id: string, props: Partial<ToasterToast>) {
  toastsStore.set(
    toastsStore.get().map((t) => (t.id === id ? { ...t, ...props } : t))
  )
}

export function dismissToast(toastId?: string) {
  const toasts = toastsStore.get()
  const targetIds = toastId ? [toastId] : toasts.map((t) => t.id)
  targetIds.forEach((id) => addToRemoveQueue(id))
  toastsStore.set(
    toasts.map((t) =>
      t.id === toastId || toastId === undefined ? { ...t, open: false } : t
    )
  )
}

export function removeToast(toastId?: string) {
  if (toastId === undefined) {
    toastsStore.set([])
    return
  }
  toastsStore.set(toastsStore.get().filter((t) => t.id !== toastId))
}

export function toast(props: Omit<ToasterToast, "id">) {
  return addToast(props)
}
