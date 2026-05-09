"use client"

import React, { type ReactNode } from "react"
import { Loader2 } from "lucide-react"
import { CodeBlock } from "@/components/ui/code-block"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

type CodeReferencePreview = {
  path: string
  relativePath: string
  filename: string
  language: string
  code: string
  startLine: number
  highlightLine: number | null
  totalLines: number
}

const ANSI_COLOR_CLASSES: Record<number, string> = {
  30: "text-zinc-400",
  31: "text-red-300",
  32: "text-emerald-300",
  33: "text-amber-300",
  34: "text-sky-300",
  35: "text-fuchsia-300",
  36: "text-cyan-300",
  37: "text-white/85",
  90: "text-zinc-500",
  91: "text-red-200",
  92: "text-emerald-200",
  93: "text-amber-200",
  94: "text-sky-200",
  95: "text-fuchsia-200",
  96: "text-cyan-200",
  97: "text-white",
}

function ansiClassFor(codes: string, currentClass: string | null) {
  const parsedCodes = codes.split(";").map((code) => Number(code || "0"))
  if (parsedCodes.includes(0) || parsedCodes.includes(39)) return null

  for (let index = parsedCodes.length - 1; index >= 0; index--) {
    const colorClass = ANSI_COLOR_CLASSES[parsedCodes[index]]
    if (colorClass) return colorClass
  }

  return currentClass
}

function renderAnsi(text: string, keyPrefix: string): ReactNode[] {
  const nodes: ReactNode[] = []
  const pattern = /\u001b\[([0-9;]*)m/g
  let activeClass: string | null = null
  let lastIndex = 0
  let match: RegExpExecArray | null

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > lastIndex) {
      const value = text.slice(lastIndex, match.index)
      nodes.push(activeClass ? (
        <span key={`${keyPrefix}-${lastIndex}`} className={activeClass}>{value}</span>
      ) : value)
    }

    activeClass = ansiClassFor(match[1], activeClass)
    lastIndex = match.index + match[0].length
  }

  if (lastIndex < text.length) {
    const value = text.slice(lastIndex)
    nodes.push(activeClass ? (
      <span key={`${keyPrefix}-${lastIndex}`} className={activeClass}>{value}</span>
    ) : value)
  }

  return nodes
}

function compactFileLabel(label: string, href: string) {
  const target = href.replace(/^file:\/\//, "")
  const lineMatch = target.match(/:(\d+)$/)
  const withoutLine = lineMatch ? target.slice(0, -lineMatch[0].length) : target
  const filename = withoutLine.split("/").filter(Boolean).at(-1)
  const visibleLabel = filename || label || href

  return lineMatch ? `${visibleLabel}:${lineMatch[1]}` : visibleLabel
}

function isLocalFileHref(href: string) {
  return href.startsWith("/") || href.startsWith("file://")
}

function renderLink(token: string, key: string) {
  const match = token.match(/^\[([^\]\n]+)\]\(([^)]+)\)$/)
  if (!match) return token

  const [, label, href] = match
  if (isLocalFileHref(href)) {
    return <CodeReferencePill key={key} label={label} href={href} />
  }

  return (
    <a
      key={key}
      href={href}
      target="_blank"
      rel="noreferrer"
      className="rounded-full border border-sky-300/20 bg-sky-300/10 px-1.5 py-0.5 text-sky-100 underline-offset-2 hover:underline"
      title={href}
    >
      {label}
    </a>
  )
}

function CodeReferencePill({ label, href }: { label: string; href: string }) {
  const [open, setOpen] = React.useState(false)
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  const [preview, setPreview] = React.useState<CodeReferencePreview | null>(null)
  const compactLabel = compactFileLabel(label, href)

  const loadPreview = React.useCallback(async () => {
    if (preview || loading) return
    setLoading(true)
    setError(null)
    try {
      const response = await fetch(`/api/code-reference?target=${encodeURIComponent(href)}`, {
        cache: "no-store",
      })
      const data = await response.json().catch(() => ({})) as Partial<CodeReferencePreview> & { error?: string }
      if (!response.ok) throw new Error(data.error || `HTTP ${response.status}`)
      setPreview(data as CodeReferencePreview)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [href, loading, preview])

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <button
        type="button"
        className="mx-0.5 inline-flex max-w-[13rem] items-center rounded-full border border-cyan-300/20 bg-cyan-300/10 px-1.5 py-0.5 font-mono text-[0.9em] leading-none text-cyan-100 align-baseline transition-colors hover:border-cyan-200/40 hover:bg-cyan-300/15"
        title={href}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={() => {
          setOpen(true)
          void loadPreview()
        }}
      >
        {loading ? <Loader2 className="mr-1 h-3 w-3 animate-spin" /> : null}
        <span className="truncate">{compactLabel}</span>
      </button>
      <DialogContent className="max-h-[88vh] overflow-hidden p-0 sm:max-w-5xl">
        <DialogHeader className="border-b border-border px-4 py-3">
          <DialogTitle className="font-mono text-sm">{compactLabel}</DialogTitle>
          <DialogDescription className="truncate font-mono text-xs">
            {preview?.relativePath || href}
          </DialogDescription>
        </DialogHeader>
        <div className="max-h-[calc(88vh-92px)] overflow-auto p-3">
          {error ? (
            <div className="rounded-md border border-red-400/20 bg-red-400/10 px-3 py-2 text-sm text-red-100">
              {error}
            </div>
          ) : preview ? (
            <CodeBlock
              language={preview.language}
              filename={preview.filename}
              code={preview.code}
              startingLineNumber={preview.startLine}
              highlightLines={preview.highlightLine ? [preview.highlightLine] : []}
            />
          ) : (
            <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Loading reference
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}

function renderInline(text: string): ReactNode[] {
  const nodes: ReactNode[] = []
  const pattern = /(\[[^\]\n]+\]\([^)]+\)|`[^`]+`|\*\*[^*]+\*\*)/g
  let lastIndex = 0
  let match: RegExpExecArray | null

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > lastIndex) nodes.push(...renderAnsi(text.slice(lastIndex, match.index), `${match.index}-text`))
    const token = match[0]
    if (token.startsWith("`")) {
      nodes.push(
        <code key={`${match.index}-code`} className="rounded border border-white/10 bg-white/5 px-1 py-0.5 font-mono text-[0.9em] text-white/85">
          {token.slice(1, -1)}
        </code>,
      )
    } else {
      nodes.push(
        token.startsWith("**") ? (
          <strong key={`${match.index}-strong`} className="font-semibold text-white">
            {token.slice(2, -2)}
          </strong>
        ) : renderLink(token, `${match.index}-link`),
      )
    }
    lastIndex = match.index + token.length
  }

  if (lastIndex < text.length) nodes.push(...renderAnsi(text.slice(lastIndex), `${lastIndex}-text`))
  return nodes
}

function renderTextLine(line: string, key: string) {
  const heading = line.match(/^(#{1,3})\s+(.+)$/)
  if (heading) {
    const size = heading[1].length === 1 ? "text-[13px]" : "text-xs"
    return (
      <div key={key} className={`${size} mt-1 font-semibold leading-snug text-white`}>
        {renderInline(heading[2])}
      </div>
    )
  }

  const bullet = line.match(/^[-*]\s+(.+)$/)
  if (bullet) {
    return (
      <div key={key} className="flex gap-2 leading-relaxed">
        <span className="mt-[0.35em] h-1 w-1 shrink-0 rounded-full bg-white/45" />
        <span className="min-w-0">{renderInline(bullet[1])}</span>
      </div>
    )
  }

  const numbered = line.match(/^(\d+)\.\s+(.+)$/)
  if (numbered) {
    return (
      <div key={key} className="flex gap-2 leading-relaxed">
        <span className="shrink-0 font-mono text-[10px] text-white/45">{numbered[1]}.</span>
        <span className="min-w-0">{renderInline(numbered[2])}</span>
      </div>
    )
  }

  const quote = line.match(/^>\s?(.+)$/)
  if (quote) {
    return (
      <div key={key} className="border-l border-white/15 pl-2 text-white/70">
        {renderInline(quote[1])}
      </div>
    )
  }

  return <div key={key} className="leading-relaxed">{renderInline(line)}</div>
}

function renderTextBlock(text: string, keyPrefix: string) {
  return text.split("\n").map((line, lineIndex) => line ? (
    renderTextLine(line, `${keyPrefix}-${lineIndex}`)
  ) : (
    <div key={`${keyPrefix}-${lineIndex}`} className="h-2" />
  ))
}

export function AssistantMessageContent({ content }: { content: string }) {
  return content.split("```").map((part, index) => {
    if (index % 2 === 1) {
      const firstNewline = part.indexOf("\n")
      const language = firstNewline > 0 ? part.slice(0, firstNewline).trim() : ""
      const code = firstNewline > 0 ? part.slice(firstNewline + 1) : part

      return (
        <div key={index} className="my-1 min-w-0 max-w-full overflow-hidden rounded-md border border-white/10 bg-black/25">
          {language ? (
            <div className="border-b border-white/10 px-2 py-1 font-mono text-[9px] uppercase tracking-wide text-white/45">
              {language}
            </div>
          ) : null}
          <pre className="max-w-full overflow-x-auto whitespace-pre-wrap break-words px-2 py-1.5 font-mono text-[10px] leading-4 text-white/85">
            <code>{renderAnsi(code.trimEnd(), `${index}-code`)}</code>
          </pre>
        </div>
      )
    }

    return renderTextBlock(part, String(index))
  })
}
