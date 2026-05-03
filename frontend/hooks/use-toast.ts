'use client'

import { useStore } from "@nanostores/react"
import type { ToastActionElement, ToastProps } from '@/components/ui/toast'
import { toastsStore, addToast, dismissToast, removeToast, hasOpenToastStore } from "@/stores/toast-store"

type ToasterToast = ToastProps & {
  id: string
  title?: string | React.ReactNode
  description?: string | React.ReactNode
  action?: ToastActionElement
}

function useToast() {
  const toasts = useStore(toastsStore)

  return {
    toasts,
    toast: (props: Omit<ToasterToast, "id">) => {
      const result = addToast(props)
      return {
        id: result.id,
        dismiss: result.dismiss,
        update: (props: Partial<ToasterToast>) => result.update(props),
      }
    },
    dismiss: (toastId?: string) => dismissToast(toastId),
  }
}

function toast(props: Omit<ToasterToast, "id">) {
  const result = addToast(props)
  return {
    id: result.id,
    dismiss: result.dismiss,
    update: (props: Partial<ToasterToast>) => result.update(props),
  }
}

export { useToast, toast }
