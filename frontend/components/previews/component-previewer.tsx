"use client"

import { useMemo, useState } from "react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { cn } from "@/lib/utils"
import {
  COMPONENT_GROUP_LABELS,
  COMPONENT_PREVIEWS,
  type ComponentPreviewGroup,
} from "./component-preview-registry"

const STATUS_TONE = {
  canonical: "border-primary/30 bg-primary/10 text-primary",
  candidate: "border-amber-500/30 bg-amber-500/10 text-amber-300",
  scaffold: "border-blue-500/30 bg-blue-500/10 text-blue-300",
  legacy: "border-destructive/30 bg-destructive/10 text-destructive",
} as const

export function ComponentPreviewer() {
  const [activeId, setActiveId] = useState(COMPONENT_PREVIEWS[0]?.id ?? "")
  const [activeGroup, setActiveGroup] = useState<ComponentPreviewGroup | "all">("all")

  const groups = useMemo(() => {
    return Array.from(new Set(COMPONENT_PREVIEWS.map((preview) => preview.group)))
  }, [])

  const visiblePreviews = useMemo(() => {
    if (activeGroup === "all") return COMPONENT_PREVIEWS
    return COMPONENT_PREVIEWS.filter((preview) => preview.group === activeGroup)
  }, [activeGroup])

  const activePreview = COMPONENT_PREVIEWS.find((preview) => preview.id === activeId) ?? visiblePreviews[0] ?? COMPONENT_PREVIEWS[0]
  const ActiveComponent = activePreview?.component

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <aside className="flex w-80 shrink-0 flex-col border-r border-border bg-card/45">
        <div className="border-b border-border p-4">
          <h1 className="text-lg font-semibold tracking-tight">Component Previewer</h1>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            Canonical surfaces first. Demos and legacy pieces should be promoted here before they are used in production.
          </p>
        </div>

        <div className="border-b border-border p-3">
          <div className="flex flex-wrap gap-1.5">
            <Button
              size="xs"
              variant={activeGroup === "all" ? "default" : "outline"}
              onClick={() => setActiveGroup("all")}
            >
              All
            </Button>
            {groups.map((group) => (
              <Button
                key={group}
                size="xs"
                variant={activeGroup === group ? "default" : "outline"}
                onClick={() => setActiveGroup(group)}
              >
                {COMPONENT_GROUP_LABELS[group]}
              </Button>
            ))}
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {visiblePreviews.map((preview) => (
            <button
              key={preview.id}
              onClick={() => setActiveId(preview.id)}
              className={cn(
                "mb-2 w-full rounded-2xl border p-3 text-left transition-colors",
                activePreview?.id === preview.id
                  ? "border-primary/40 bg-primary/10"
                  : "border-border bg-background/45 hover:bg-secondary/60",
              )}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{preview.title}</div>
                  <div className="mt-1 text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                    {COMPONENT_GROUP_LABELS[preview.group]}
                  </div>
                </div>
                <Badge variant="outline" className={cn("shrink-0 text-[10px]", STATUS_TONE[preview.status])}>
                  {preview.status}
                </Badge>
              </div>
              <p className="mt-2 line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                {preview.description}
              </p>
            </button>
          ))}
        </div>
      </aside>

      <main className="min-w-0 flex-1 overflow-hidden">
        <div className="flex h-16 items-center justify-between border-b border-border bg-card/35 px-5">
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold">{activePreview?.title}</div>
            <div className="truncate text-xs text-muted-foreground">{activePreview?.description}</div>
          </div>
          {activePreview && (
            <Badge variant="outline" className={cn("text-[10px]", STATUS_TONE[activePreview.status])}>
              {activePreview.status}
            </Badge>
          )}
        </div>

        <div className="h-[calc(100vh-4rem)] overflow-auto p-5">
          {ActiveComponent ? (
            <Card className="min-h-full border-border bg-card/35 shadow-none">
              <CardContent className="p-4">
                <ActiveComponent />
              </CardContent>
            </Card>
          ) : (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              No preview selected.
            </div>
          )}
        </div>
      </main>
    </div>
  )
}
