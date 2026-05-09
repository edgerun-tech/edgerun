"use client"

import { useState, type FormEvent } from "react"
import { useStore } from "@nanostores/react"
import { CheckSquare, Plus, X } from "lucide-react"
import {
  addProjectChecklistItem,
  projectChecklistStore,
  removeProjectChecklistItem,
  toggleProjectChecklistItem,
} from "@/stores/project-checklist-store"
import { cn } from "@/lib/utils"

export function ProjectChecklist() {
  const items = useStore(projectChecklistStore)
  const [draft, setDraft] = useState("")
  const completeCount = items.filter((item) => item.done).length

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    addProjectChecklistItem(draft)
    setDraft("")
  }

  return (
    <div className="fixed bottom-24 right-4 z-40 w-[min(320px,calc(100vw-2rem))] text-foreground sm:right-5">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <CheckSquare className="h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <div className="truncate text-xs font-semibold">Project Checklist</div>
            <div className="font-mono text-[10px] text-muted-foreground">{completeCount}/{items.length} done</div>
          </div>
        </div>
      </div>

      <div className="max-h-[min(38vh,280px)] space-y-1 overflow-y-auto pr-1 scrollbar-none">
        {items.map((item) => (
          <div key={item.id} className="group flex items-center gap-2 rounded-md px-1 py-1">
            <input
              type="checkbox"
              checked={item.done}
              onChange={() => toggleProjectChecklistItem(item.id)}
              className="h-3.5 w-3.5 shrink-0"
              aria-label={item.label}
            />
            <button
              type="button"
              onClick={() => toggleProjectChecklistItem(item.id)}
              className={cn(
                "min-w-0 flex-1 truncate text-left text-xs",
                item.done ? "text-muted-foreground line-through" : "text-foreground",
              )}
              title={item.label}
            >
              {item.label}
            </button>
            <button
              type="button"
              onClick={() => removeProjectChecklistItem(item.id)}
              className="flex h-5 w-5 shrink-0 items-center justify-center rounded text-muted-foreground opacity-0 transition-opacity hover:bg-secondary hover:text-foreground group-hover:opacity-100"
              aria-label={`Remove ${item.label}`}
            >
              <X className="h-3 w-3" />
            </button>
          </div>
        ))}
      </div>

      <form onSubmit={submit} className="mt-2 flex items-center gap-1.5 pt-1">
        <input
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Add item"
          className="min-w-0 flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground/60"
        />
        <button
          type="submit"
          disabled={!draft.trim()}
          className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md border border-border text-muted-foreground hover:bg-secondary hover:text-foreground disabled:opacity-35"
          aria-label="Add checklist item"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </form>
    </div>
  )
}
