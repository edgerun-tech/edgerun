"use client"

import { persistentAtom } from "@nanostores/persistent"

export type ProjectChecklistItem = {
  id: string
  label: string
  done: boolean
}

const DEFAULT_ITEMS: ProjectChecklistItem[] = [
  { id: "desktop-shell", label: "Stabilize desktop shell", done: false },
  { id: "demo-cleanup", label: "Clean demo leftovers", done: false },
  { id: "assistant-ux", label: "Harden assistant UX", done: false },
  { id: "split-work", label: "Split current work into commits", done: false },
  { id: "node-identity", label: "Build real node identity command", done: false },
]

export const projectChecklistStore = persistentAtom<ProjectChecklistItem[]>(
  "edgerun:project-checklist",
  DEFAULT_ITEMS,
  {
    encode: JSON.stringify,
    decode: (value) => {
      if (!value) return DEFAULT_ITEMS
      const parsed = JSON.parse(value) as unknown
      if (!Array.isArray(parsed)) return DEFAULT_ITEMS
      const items = parsed.filter((item): item is ProjectChecklistItem => (
        item &&
        typeof item === "object" &&
        "id" in item &&
        "label" in item &&
        "done" in item &&
        typeof item.id === "string" &&
        typeof item.label === "string" &&
        typeof item.done === "boolean"
      ))
      return items.length ? items : DEFAULT_ITEMS
    },
  },
)

export function toggleProjectChecklistItem(id: string) {
  projectChecklistStore.set(projectChecklistStore.get().map((item) => (
    item.id === id ? { ...item, done: !item.done } : item
  )))
}

export function findProjectChecklistItem(query: string) {
  const normalized = query.trim().toLowerCase()
  if (!normalized) return null
  return projectChecklistStore.get().find((item) => (
    item.id.toLowerCase() === normalized ||
    item.label.toLowerCase() === normalized ||
    item.label.toLowerCase().includes(normalized)
  )) ?? null
}

export function setProjectChecklistItemDone(id: string, done: boolean) {
  projectChecklistStore.set(projectChecklistStore.get().map((item) => (
    item.id === id ? { ...item, done } : item
  )))
}

export function addProjectChecklistItem(label: string) {
  const trimmed = label.trim()
  if (!trimmed) return
  projectChecklistStore.set([
    ...projectChecklistStore.get(),
    {
      id: `item-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      label: trimmed,
      done: false,
    },
  ])
}

export function removeProjectChecklistItem(id: string) {
  projectChecklistStore.set(projectChecklistStore.get().filter((item) => item.id !== id))
}

export function formatProjectChecklist() {
  const items = projectChecklistStore.get()
  if (!items.length) return "Checklist is empty."
  return [
    "Project checklist",
    ...items.map((item) => `${item.done ? "[x]" : "[ ]"} ${item.label}`),
  ].join("\n")
}
