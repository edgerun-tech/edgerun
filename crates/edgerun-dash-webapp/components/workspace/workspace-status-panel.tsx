"use client"

import { useState, useEffect, useCallback } from "react"
import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import {
  gitStatus,
  otherAgentStatus,
  getGitSummary,
  getOtherAgentSummary,
  updateGitStatus,
  detectOtherAgents,
  type GitStatus,
  type OtherAgentStatus,
} from "@/platform/assistant/assistant-git-awareness"
import {
  verificationStatus,
  verificationResult,
  getVerificationSummary,
  getVerificationErrors,
  getVerificationWarnings,
} from "@/platform/assistant/assistant-verification"
import {
  policyStatus,
  dependencyPolicyStore,
} from "@/platform/assistant/assistant-dependency-policy"
import {
  selfReviewStore,
  needsReview,
  performSelfReview,
  incrementTurn,
  recordUserCorrection,
} from "@/platform/assistant/assistant-self-review"
import {
  memoryStore,
  contextAge,
  isContextStale,
  getMemorySummary,
} from "@/platform/assistant/assistant-memory"
import {
  GitBranch,
  GitCommit,
  GitMerge,
  CheckCircle2,
  AlertCircle,
  Circle,
  Clock,
  Brain,
  MessageSquare,
  ListTodo,
  Zap,
  Terminal,
  Shield,
  Eye,
  EyeOff,
  ChevronDown,
  ChevronUp,
  RefreshCw,
} from "lucide-react"

interface WorkspaceStatusPanelProps {
  className?: string
}

export function WorkspaceStatusPanel({ className }: WorkspaceStatusPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true)
  const [isHovered, setIsHovered] = useState(false)
  const [currentTime, setCurrentTime] = useState(Date.now())

  const git = useStore(gitStatus)
  const otherAgents = useStore(otherAgentStatus)
  const verification = useStore(verificationStatus)
  const policy = useStore(policyStatus)
  const review = useStore(selfReviewStore)
  const memContextAge = useStore(contextAge)
  const contextStale = useStore(isContextStale)
  const reviewNeeded = useStore(needsReview)

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Date.now())
      incrementTurn()
    }, 5000)
    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    if (reviewNeeded) {
      performSelfReview()
    }
  }, [reviewNeeded])

  const gitSummary = getGitSummary()
  const otherAgentSummary = getOtherAgentSummary()
  const verificationSummary = getVerificationSummary()
  const verificationErrors = getVerificationErrors()
  const verificationWarnings = getVerificationWarnings()
  const memorySummary = getMemorySummary()

  const formatAge = (ageMs: number) => {
    if (ageMs < 60000) return `${Math.floor(ageMs / 1000)}s`
    if (ageMs < 3600000) return `${Math.floor(ageMs / 60000)}m`
    return `${Math.floor(ageMs / 3600000)}h`
  }

  const getGitIcon = () => {
    switch (git) {
      case "clean":
        return <CheckCircle2 className="h-3 w-3 text-green-500" />
      case "dirty":
        return <GitCommit className="h-3 w-3 text-yellow-500" />
      default:
        return <Circle className="h-3 w-3 text-muted-foreground" />
    }
  }

  const getVerificationIcon = () => {
    switch (verification) {
      case "passed":
        return <CheckCircle2 className="h-3 w-3 text-green-500" />
      case "failed":
        return <AlertCircle className="h-3 w-3 text-red-500" />
      case "running":
        return <RefreshCw className="h-3 w-3 text-blue-500 animate-spin" />
      default:
        return <Circle className="h-3 w-3 text-muted-foreground" />
    }
  }

  const getPolicyIcon = () => {
    switch (policy) {
      case "no_new_deps":
        return <Shield className="h-3 w-3 text-green-500" />
      case "approval_required":
        return <AlertCircle className="h-3 w-3 text-yellow-500" />
      case "pending_review":
        return <Clock className="h-3 w-3 text-orange-500" />
    }
  }

  const getAgentIcon = () => {
    switch (otherAgents) {
      case "detected":
        return <GitMerge className="h-3 w-3 text-purple-500" />
      case "not_detected":
        return <Eye className="h-3 w-3 text-green-500" />
      default:
        return <EyeOff className="h-3 w-3 text-muted-foreground" />
    }
  }

  return (
    <div
      className={cn(
        "pointer-events-auto fixed bottom-4 left-4 z-40 flex flex-col gap-2 rounded-lg border border-[var(--border)] bg-[var(--bg)]/90 backdrop-blur-sm transition-opacity duration-200",
        isHovered ? "opacity-100" : "opacity-80",
        className
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between gap-2 px-3 py-2 text-xs font-medium hover:bg-[var(--bg-hover)]"
      >
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-primary" />
          <span>Workspace Status</span>
        </div>
        {isExpanded ? (
          <ChevronDown className="h-3 w-3" />
        ) : (
          <ChevronUp className="h-3 w-3" />
        )}
      </button>

      {isExpanded && (
        <div className="flex flex-col gap-1 px-3 pb-3">
          <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-[10px]">
            <div className="flex items-center gap-1.5">
              {getGitIcon()}
              <span className="text-muted-foreground">Git:</span>
              <span className={cn(
                "font-mono",
                git === "clean" && "text-green-500",
                git === "dirty" && "text-yellow-500"
              )}>
                {git}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              {getAgentIcon()}
              <span className="text-muted-foreground">Agents:</span>
              <span className={cn(
                "font-mono",
                otherAgents === "detected" && "text-purple-500",
                otherAgents === "not_detected" && "text-green-500"
              )}>
                {otherAgents === "not_detected" ? "none" : otherAgents}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              {getVerificationIcon()}
              <span className="text-muted-foreground">Verify:</span>
              <span className={cn(
                "font-mono",
                verification === "passed" && "text-green-500",
                verification === "failed" && "text-red-500",
                verification === "running" && "text-blue-500"
              )}>
                {verification}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              {getPolicyIcon()}
              <span className="text-muted-foreground">Deps:</span>
              <span className="font-mono">
                {policy === "no_new_deps" ? "clean" : policy.replace("_", " ")}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              <Clock className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Ctx:</span>
              <span className={cn(
                "font-mono",
                contextStale ? "text-yellow-500" : "text-green-500"
              )}>
                {formatAge(memContextAge)}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              <ListTodo className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Turn:</span>
              <span className="font-mono">{review.turnCount}</span>
            </div>
          </div>

          {verificationErrors.length > 0 && (
            <div className="mt-2 rounded border border-red-500/30 bg-red-500/10 px-2 py-1">
              <div className="text-[10px] text-red-400">Errors:</div>
              <div className="text-[9px] text-red-300">
                {verificationErrors.slice(0, 2).join(", ")}
              </div>
            </div>
          )}

          {verificationWarnings.length > 0 && (
            <div className="mt-2 rounded border border-yellow-500/30 bg-yellow-500/10 px-2 py-1">
              <div className="text-[10px] text-yellow-400">Warnings:</div>
              <div className="text-[9px] text-yellow-300">
                {verificationWarnings.slice(0, 2).join(", ")}
              </div>
            </div>
          )}

          {memorySummary && (
            <div className="mt-2 flex items-center gap-1 text-[9px] text-muted-foreground">
              <Brain className="h-3 w-3" />
              <span>{memorySummary}</span>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export function useWorkspaceAwareness() {
  const refreshGitStatus = useCallback(() => {
    if (typeof window === "undefined") return

    updateGitStatus("unknown")
  }, [])

  return { refreshGitStatus }
}
