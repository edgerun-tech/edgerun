"use client"

import { useState, useRef, useCallback, useEffect } from "react"
import { cn } from "@/lib/utils"
import { Send, Sparkles, Loader2, X, Lightbulb } from "lucide-react"

export interface Suggestion {
  id: string
  label: string
  description?: string
  action: () => void
}

interface WorkspacePromptInputProps {
  className?: string
  onSubmit: (value: string) => void
  onSuggestionsClick?: (suggestion: Suggestion) => void
  suggestions?: Suggestion[]
  placeholder?: string
  isLoading?: boolean
}

export function WorkspacePromptInput({
  className,
  onSubmit,
  onSuggestionsClick,
  suggestions = [],
  placeholder = "Ask the assistant...",
  isLoading = false,
}: WorkspacePromptInputProps) {
  const [inputValue, setInputValue] = useState("")
  const [isHovered, setIsHovered] = useState(false)
  const [showSuggestions, setShowSuggestions] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const handleSubmit = useCallback(() => {
    const text = inputValue.trim()
    if (!text || isLoading) return
    onSubmit(text)
    setInputValue("")
  }, [inputValue, isLoading, onSubmit])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSubmit()
    }
    if (e.key === "Escape") {
      setInputValue("")
      setShowSuggestions(false)
    }
  }

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 120)}px`
    }
  }, [inputValue])

  const handleSuggestionClick = (suggestion: Suggestion) => {
    onSuggestionsClick?.(suggestion)
    suggestion.action()
    setShowSuggestions(false)
  }

  return (
    <div
      className={cn(
        "fixed bottom-20 left-1/2 z-50 flex w-full max-w-2xl -translate-x-1/2 flex-col items-center gap-2 transition-opacity duration-200",
        isHovered ? "opacity-100" : "opacity-50",
        className
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {showSuggestions && suggestions.length > 0 && (
        <div className="flex w-full flex-wrap gap-1 rounded-lg border border-[var(--border)] bg-[var(--bg)] p-2 shadow-lg">
          {suggestions.map((suggestion) => (
            <button
              key={suggestion.id}
              onClick={() => handleSuggestionClick(suggestion)}
              className="flex items-center gap-1.5 rounded-md bg-[var(--bg-hover)] px-2 py-1 text-xs text-foreground hover:bg-[var(--accent)] hover:text-[var(--accent-fg)]"
            >
              <Lightbulb className="h-3 w-3 text-yellow-500" />
              <span>{suggestion.label}</span>
            </button>
          ))}
        </div>
      )}

      <div className="flex w-full items-end gap-2 rounded-xl border border-[var(--border)] bg-[var(--bg)] p-2 shadow-lg">
        <button
          onClick={() => setShowSuggestions(!showSuggestions)}
          className={cn(
            "flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg transition-colors",
            showSuggestions
              ? "bg-primary text-primary-foreground"
              : "bg-[var(--bg-hover)] text-muted-foreground hover:text-foreground"
          )}
          title="Suggestions"
        >
          <Sparkles className="h-4 w-4" />
        </button>

        <div className="flex-1">
          <textarea
            ref={textareaRef}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            className="max-h-32 w-full resize-none bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
            rows={1}
            disabled={isLoading}
          />
        </div>

        {inputValue && (
          <button
            onClick={() => setInputValue("")}
            className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg text-muted-foreground hover:bg-[var(--bg-hover)] hover:text-foreground"
            title="Clear"
          >
            <X className="h-4 w-4" />
          </button>
        )}

        <button
          onClick={handleSubmit}
          disabled={!inputValue.trim() || isLoading}
          className={cn(
            "flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg transition-colors",
            inputValue.trim() && !isLoading
              ? "bg-primary text-primary-foreground hover:bg-primary/90"
              : "bg-[var(--bg-hover)] text-muted-foreground"
          )}
          title="Send"
        >
          {isLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Send className="h-4 w-4" />
          )}
        </button>
      </div>
    </div>
  )
}
