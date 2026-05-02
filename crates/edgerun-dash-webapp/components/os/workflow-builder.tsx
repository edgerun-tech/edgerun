"use client"

import { useEffect, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  workflowStore,
  createWorkflow,
  updateWorkflow,
  deleteWorkflow,
  addStage,
  addAction,
  removeAction,
  removeStage,
  executeWorkflow,
  type Workflow,
  type WorkflowStage,
  type WorkflowAction,
  type ActionType,
  type WorkflowExecution,
} from "@/stores/workflow-store"

const ACTION_TYPES: { type: ActionType; label: string; icon: string; defaultConfig: Record<string, unknown> }[] = [
  { type: "log", label: "Log", icon: "📝", defaultConfig: { message: "" } },
  { type: "delay", label: "Delay", icon: "⏱️", defaultConfig: { duration: 5 } },
  { type: "http", label: "HTTP Request", icon: "🌐", defaultConfig: { url: "", method: "GET" } },
  { type: "notify", label: "Notification", icon: "🔔", defaultConfig: { title: "", message: "" } },
  { type: "deploy", label: "Deploy", icon: "🚀", defaultConfig: { target: "", image: "" } },
  { type: "condition", label: "Condition", icon: "❓", defaultConfig: { condition: "" } },
  { type: "transform", label: "Transform", icon: "🔄", defaultConfig: { operation: "", field: "" } },
]

export function WorkflowBuilder({ onClose }: { onClose: () => void }) {
  const store = useStore(workflowStore)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [showNewDialog, setShowNewDialog] = useState(false)
  const [newName, setNewName] = useState("")
  const [newDesc, setNewDesc] = useState("")
  const [showActionDialog, setShowActionDialog] = useState<{ stageId: string; type: ActionType } | null>(null)
  const [actionConfig, setActionConfig] = useState<Record<string, unknown>>({})
  const [executing, setExecuting] = useState<string | null>(null)
  
  const selected = selectedId ? Array.from(store.workflows.values()).find(w => w.workflowId === selectedId) : null
  
  useEffect(() => {
    if (!selectedId && store.workflows.size > 0) {
      setSelectedId(Array.from(store.workflows.values())[0].workflowId)
    }
  }, [store.workflows, selectedId])
  
  const handleCreate = () => {
    if (!newName.trim()) return
    const wf = createWorkflow(newName.trim(), newDesc.trim(), 'manual')
    setSelectedId(wf)
    setShowNewDialog(false)
    setNewName("")
    setNewDesc("")
  }
  
  const handleExecute = async (id: string) => {
    setExecuting(id)
    try {
      await executeWorkflow(id)
    } finally {
      setExecuting(null)
    }
  }
  
  return (
    <>
      <div className="flex h-full">
        {/* Sidebar - Workflow List */}
        <div className="w-56 border-r border-[var(--border)] flex flex-col">
          <div className="p-2 border-b border-[var(--border)]">
            <button
              onClick={() => setShowNewDialog(true)}
              className="w-full py-1.5 px-3 bg-[var(--accent)] text-[var(--accent-fg)] rounded text-sm font-medium hover:opacity-90"
            >
              + New Workflow
            </button>
          </div>
          
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {Array.from(store.workflows.values()).map(wf => (
              <div
                key={wf.workflowId}
                onClick={() => setSelectedId(wf.workflowId)}
                className={`p-2 rounded cursor-pointer text-sm ${
                  selectedId === wf.workflowId 
                    ? "bg-[var(--accent)]/20 border border-[var(--accent)]" 
                    : "hover:bg-[var(--bg-hover)]"
                }`}
              >
                <div className="font-medium truncate">{wf.name}</div>
                <div className="text-[var(--text-muted)] text-xs truncate">
                  {wf.stages.length} stages • {wf.isEnabled ? "enabled" : "disabled"}
                </div>
              </div>
            ))}
            
            {store.workflows.size === 0 && (
              <div className="text-center text-[var(--text-muted)] text-sm p-4">
                No workflows yet.<br />Create one to get started.
              </div>
            )}
          </div>
        </div>
        
        {/* Main Content */}
        <div className="flex-1 flex flex-col">
          {selected ? (
            <>
              {/* Toolbar */}
              <div className="p-3 border-b border-[var(--border)] flex items-center justify-between">
                <div>
                  <input
                    type="text"
                    value={selected.name}
                    onChange={e => updateWorkflow(selected.workflowId, { name: e.target.value })}
                    className="font-medium bg-transparent border-none outline-none text-lg"
                  />
                  <input
                    type="text"
                    value={selected.description}
                    onChange={e => updateWorkflow(selected.workflowId, { description: e.target.value })}
                    placeholder="Description..."
                    className="block w-full mt-1 text-sm text-[var(--text-muted)] bg-transparent border-none outline-none"
                  />
                </div>
                
                <div className="flex gap-2">
                  <button
                    onClick={() => updateWorkflow(selected.workflowId, { isEnabled: !selected.isEnabled })}
                    className={`px-3 py-1 rounded text-sm ${
                      selected.isEnabled 
                        ? "bg-green-600/20 text-green-400" 
                        : "bg-gray-600/20 text-gray-400"
                    }`}
                  >
                    {selected.isEnabled ? "Enabled" : "Disabled"}
                  </button>
                  
                  <button
                    onClick={() => handleExecute(selected.workflowId)}
                    disabled={executing === selected.workflowId}
                    className="px-3 py-1 bg-[var(--accent)] text-[var(--accent-fg)] rounded text-sm disabled:opacity-50"
                  >
                    {executing === selected.workflowId ? "Running..." : "▶ Run"}
                  </button>
                  
                  <button
                    onClick={() => {
                      deleteWorkflow(selected.workflowId)
                      setSelectedId(Array.from(store.workflows.values()).find(w => w.workflowId !== selected.workflowId)?.workflowId || null)
                    }}
                    className="px-3 py-1 bg-red-600/20 text-red-400 rounded text-sm"
                  >
                    Delete
                  </button>
                </div>
              </div>
              
              {/* Stages */}
              <div className="flex-1 overflow-x-auto p-4">
                <div className="flex gap-4 min-w-max">
                  {selected.stages.map((stage, idx) => (
                    <StageCard
                      key={stage.stageId}
                      stage={stage}
                      workflowId={selected.workflowId}
                      isLast={idx === selected.stages.length - 1}
                      onAddAction={(type) => setShowActionDialog({ stageId: stage.stageId, type })}
                      onRemoveStage={() => removeStage(selected.workflowId, stage.stageId)}
                      onRemoveAction={(actionId) => removeAction(selected.workflowId, stage.stageId, actionId)}
                    />
                  ))}
                  
                  <button
                    onClick={() => {
                      const name = `Stage ${selected.stages.length + 1}`
                      addStage(selected.workflowId, name)
                    }}
                    className="w-48 h-32 border-2 border-dashed border-[var(--border)] rounded-lg flex items-center justify-center text-[var(--text-muted)] hover:border-[var(--accent)] hover:text-[var(--accent)]"
                  >
                    + Add Stage
                  </button>
                </div>
              </div>
              
              {/* Meta */}
              <div className="p-3 border-t border-[var(--border)]">
                <div className="text-xs text-[var(--text-muted)]">
                  Created {new Date(selected.createdAt).toLocaleDateString()} · {selected.stages.length} stage(s)
                </div>
              </div>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center text-[var(--text-muted)]">
              Select or create a workflow to get started
            </div>
          )}
        </div>
      </div>
      
      {/* New Workflow Dialog */}
      {showNewDialog && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-[var(--bg)] border border-[var(--border)] rounded-lg p-4 w-80">
            <h3 className="font-medium mb-3">New Workflow</h3>
            <input
              type="text"
              placeholder="Workflow name"
              value={newName}
              onChange={e => setNewName(e.target.value)}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded mb-2 outline-none"
              autoFocus
            />
            <input
              type="text"
              placeholder="Description (optional)"
              value={newDesc}
              onChange={e => setNewDesc(e.target.value)}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded mb-3 outline-none"
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowNewDialog(false)}
                className="px-3 py-1.5 text-sm text-[var(--text-muted)] hover:text-[var(--text)]"
              >
                Cancel
              </button>
              <button
                onClick={handleCreate}
                disabled={!newName.trim()}
                className="px-3 py-1.5 bg-[var(--accent)] text-[var(--accent-fg)] rounded text-sm disabled:opacity-50"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}
      
      {/* Action Dialog */}
      {showActionDialog && (
        <ActionConfigDialog
          type={showActionDialog.type}
          config={actionConfig}
          onChange={setActionConfig}
          onSave={() => {
            if (!showActionDialog) return
            const actionType = ACTION_TYPES.find(t => t.type === showActionDialog.type)
            addAction(selected!.workflowId, showActionDialog.stageId, {
              type: showActionDialog.type,
              name: `${actionType?.label || showActionDialog.type} Action`,
              config: actionConfig,
            })
            setShowActionDialog(null)
            setActionConfig({})
          }}
          onCancel={() => {
            setShowActionDialog(null)
            setActionConfig({})
          }}
        />
      )}
      </>
  )
}

function StageCard({
  stage,
  workflowId,
  isLast,
  onAddAction,
  onRemoveStage,
  onRemoveAction,
}: {
  stage: WorkflowStage
  workflowId: string
  isLast: boolean
  onAddAction: (type: ActionType) => void
  onRemoveStage: () => void
  onRemoveAction: (actionId: string) => void
}) {
  const [showMenu, setShowMenu] = useState(false)
  
  return (
    <div className="w-56 bg-[var(--bg-hover)] rounded-lg border border-[var(--border)]">
      <div className="p-2 border-b border-[var(--border)] flex items-center justify-between">
        <span className="font-medium text-sm">{stage.name}</span>
        <div className="flex gap-1">
          <button
            onClick={() => setShowMenu(true)}
            className="text-[var(--text-muted)] hover:text-[var(--text)]"
          >
            +
          </button>
          {!isLast && stage.actions.length === 0 && (
            <button
              onClick={onRemoveStage}
              className="text-[var(--text-muted)] hover:text-red-400"
            >
              ×
            </button>
          )}
        </div>
        
        {showMenu && (
          <div className="absolute mt-1 bg-[var(--bg)] border border-[var(--border)] rounded shadow-lg z-10">
            {ACTION_TYPES.map(at => (
              <button
                key={at.type}
                onClick={() => {
                  onAddAction(at.type)
                  setShowMenu(false)
                }}
                className="block w-full px-3 py-1.5 text-left text-sm hover:bg-[var(--bg-hover)]"
              >
                {at.icon} {at.label}
              </button>
            ))}
          </div>
        )}
      </div>
      
      <div className="p-2 space-y-1 min-h-[60px]">
        {stage.actions.map(action => (
          <div
            key={action.actionId}
            className="p-2 bg-[var(--bg)] rounded border border-[var(--border)] text-sm flex items-center justify-between group"
          >
            <div>
              <span className="text-xs">{ACTION_TYPES.find(t => t.type === action.type)?.icon}</span>
              <span className="ml-1">{action.name}</span>
            </div>
            <button
              onClick={() => onRemoveAction(action.actionId)}
              className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-300"
            >
              ×
            </button>
          </div>
        ))}
        
        {stage.actions.length === 0 && (
          <div className="text-center text-[var(--text-muted)] text-xs py-2">
            No actions
          </div>
        )}
      </div>
      
      {!isLast && (
        <div className="absolute right-0 top-1/2 -translate-y-1/2 translate-x-full">
          <span className="text-[var(--text-muted)]">→</span>
        </div>
      )}
    </div>
  )
}

function ActionConfigDialog({
  type,
  config,
  onChange,
  onSave,
  onCancel,
}: {
  type: ActionType
  config: Record<string, unknown>
  onChange: (config: Record<string, unknown>) => void
  onSave: () => void
  onCancel: () => void
}) {
  const actionType = ACTION_TYPES.find(t => t.type === type)
  
  const renderFields = () => {
    switch (type) {
      case "log":
        return (
          <input
            type="text"
            placeholder="Message to log"
            value={(config.message as string) || ""}
            onChange={e => onChange({ ...config, message: e.target.value })}
            className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
          />
        )
      case "delay":
        return (
          <input
            type="number"
            placeholder="Duration in seconds"
            value={(config.duration as string) || "5"}
            onChange={e => onChange({ ...config, duration: e.target.value })}
            className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
          />
        )
      case "http":
        return (
          <div className="space-y-2">
            <input
              type="text"
              placeholder="URL"
              value={(config.url as string) || ""}
              onChange={e => onChange({ ...config, url: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
            <select
              value={(config.method as string) || "GET"}
              onChange={e => onChange({ ...config, method: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            >
              <option value="GET">GET</option>
              <option value="POST">POST</option>
              <option value="PUT">PUT</option>
              <option value="DELETE">DELETE</option>
            </select>
          </div>
        )
      case "notify":
        return (
          <div className="space-y-2">
            <input
              type="text"
              placeholder="Title"
              value={(config.title as string) || ""}
              onChange={e => onChange({ ...config, title: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
            <input
              type="text"
              placeholder="Message"
              value={(config.message as string) || ""}
              onChange={e => onChange({ ...config, message: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
          </div>
        )
      case "deploy":
        return (
          <div className="space-y-2">
            <input
              type="text"
              placeholder="Target"
              value={(config.target as string) || ""}
              onChange={e => onChange({ ...config, target: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
            <input
              type="text"
              placeholder="Image"
              value={(config.image as string) || ""}
              onChange={e => onChange({ ...config, image: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
          </div>
        )
      case "condition":
        return (
          <input
            type="text"
            placeholder="Condition (e.g. ${var} === 'value')"
            value={(config.condition as string) || ""}
            onChange={e => onChange({ ...config, condition: e.target.value })}
            className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
          />
        )
      case "transform":
        return (
          <div className="space-y-2">
            <input
              type="text"
              placeholder="Operation (map, filter, etc.)"
              value={(config.operation as string) || ""}
              onChange={e => onChange({ ...config, operation: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
            <input
              type="text"
              placeholder="Field"
              value={(config.field as string) || ""}
              onChange={e => onChange({ ...config, field: e.target.value })}
              className="w-full px-3 py-2 bg-[var(--bg-hover)] rounded outline-none"
            />
          </div>
        )
      default:
        return null
    }
  }
  
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-[var(--bg)] border border-[var(--border)] rounded-lg p-4 w-96">
        <h3 className="font-medium mb-3">{actionType?.icon} {actionType?.label} Action</h3>
        
        {renderFields()}
        
        <div className="flex justify-end gap-2 mt-4">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 text-sm text-[var(--text-muted)] hover:text-[var(--text)]"
          >
            Cancel
          </button>
          <button
            onClick={onSave}
            className="px-3 py-1.5 bg-[var(--accent)] text-[var(--accent-fg)] rounded text-sm"
          >
            Add Action
          </button>
        </div>
      </div>
    </div>
  )
}

export function WorkflowStatus({ execution }: { execution: WorkflowExecution }) {
  const statusColors: Record<string, string> = {
    running: "text-yellow-400",
    success: "text-green-400",
    failed: "text-red-400",
    cancelled: "text-gray-500",
  }
  
  return (
    <div className="text-sm">
      <div className={`font-medium ${statusColors[execution.status] ?? "text-gray-400"}`}>
        {execution.status === "running" ? "⚡ Running" : 
         execution.status === "success" ? "✓ Success" :
         execution.status === "failed" ? "✗ Failed" :
         execution.status === "cancelled" ? "○ Cancelled" : "○ Idle"}
      </div>
      <div className="text-xs text-[var(--text-muted)] mt-1">
        {execution.stageResults.length} stage(s) · Started {new Date(execution.startedAt).toLocaleTimeString()}
      </div>
    </div>
  )
}