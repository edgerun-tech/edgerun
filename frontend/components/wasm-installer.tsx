import { useState, useCallback } from "react"
import { Upload, Link2, Loader2, Check, AlertCircle } from "lucide-react"
import { cn } from "@/lib/utils"

export interface WasmInstallResult {
  name: string
  wasmUrl: string
  wasmBytes: Uint8Array
}

interface WasmInstallerProps {
  onInstall: (result: WasmInstallResult) => void
  onCancel: () => void
}

export function WasmInstaller({ onInstall, onCancel }: WasmInstallerProps) {
  const [mode, setMode] = useState<"upload" | "url">("upload")
  const [url, setUrl] = useState("")
  const [name, setName] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  const handleFileUpload = useCallback(async (file: File) => {
    if (!file.name.endsWith(".wasm")) {
      setError("File must be a .wasm module")
      return
    }
    setLoading(true)
    setError(null)
    try {
      const bytes = new Uint8Array(await file.arrayBuffer())
      if (bytes.length < 4 || bytes[0] !== 0x00 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) {
        setError("Invalid WASM magic bytes")
        setLoading(false)
        return
      }
      const appName = name || file.name.replace(/\.wasm$/, "")
      const blobUrl = URL.createObjectURL(new Blob([bytes], { type: "application/wasm" }))
      setSuccess(true)
      setTimeout(() => {
        onInstall({ name: appName, wasmUrl: blobUrl, wasmBytes: bytes })
      }, 500)
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to read file")
    } finally {
      setLoading(false)
    }
  }, [name, onInstall])

  const handleUrlFetch = useCallback(async () => {
    if (!url) {
      setError("Enter a URL")
      return
    }
    setLoading(true)
    setError(null)
    try {
      const resp = await fetch(url)
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
      const bytes = new Uint8Array(await resp.arrayBuffer())
      if (bytes.length < 4 || bytes[0] !== 0x00 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) {
        setError("Remote file is not a valid WASM module")
        setLoading(false)
        return
      }
      const appName = name || new URL(url).pathname.split("/").pop()?.replace(/\.wasm$/, "") || "wasm-app"
      setSuccess(true)
      setTimeout(() => {
        onInstall({ name: appName, wasmUrl: url, wasmBytes: bytes })
      }, 500)
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to fetch WASM")
    } finally {
      setLoading(false)
    }
  }, [url, name, onInstall])

  return (
    <div className="flex h-full flex-col p-4">
      <h2 className="mb-1 text-sm font-semibold text-foreground">Install WASM App</h2>
      <p className="mb-4 text-xs text-muted-foreground">Upload a .wasm file or fetch from URL</p>

      <div className="mb-4 flex gap-2">
        <button
          onClick={() => setMode("upload")}
          className={cn(
            "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
            mode === "upload"
              ? "bg-primary text-primary-foreground"
              : "bg-secondary text-secondary-foreground hover:bg-secondary/80"
          )}
        >
          <Upload className="h-3.5 w-3.5" />
          Upload
        </button>
        <button
          onClick={() => setMode("url")}
          className={cn(
            "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
            mode === "url"
              ? "bg-primary text-primary-foreground"
              : "bg-secondary text-secondary-foreground hover:bg-secondary/80"
          )}
        >
          <Link2 className="h-3.5 w-3.5" />
          URL
        </button>
      </div>

      <div className="mb-3">
        <label className="mb-1 block text-xs font-medium text-muted-foreground">App Name</label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="my-wasm-app"
          className="w-full rounded-md border border-border bg-secondary px-2 py-1.5 text-xs text-foreground placeholder:text-muted-foreground/50 outline-none focus:border-primary/50"
        />
      </div>

      {mode === "upload" ? (
        <label className="flex cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed border-border bg-secondary/30 p-8 transition-colors hover:border-primary/50 hover:bg-secondary/50">
          <Upload className="mb-2 h-8 w-8 text-muted-foreground" />
          <span className="text-xs text-muted-foreground">Click to upload .wasm file</span>
          <input
            type="file"
            accept=".wasm"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0]
              if (file) handleFileUpload(file)
            }}
          />
        </label>
      ) : (
        <div className="flex gap-2">
          <input
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://example.com/app.wasm"
            className="flex-1 rounded-md border border-border bg-secondary px-2 py-1.5 text-xs text-foreground placeholder:text-muted-foreground/50 outline-none focus:border-primary/50"
            onKeyDown={(e) => { if (e.key === "Enter") handleUrlFetch() }}
          />
          <button
            onClick={handleUrlFetch}
            disabled={loading}
            className={cn(
              "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
              loading
                ? "bg-primary/50 text-primary-foreground/70"
                : "bg-primary text-primary-foreground hover:bg-primary/90"
            )}
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Link2 className="h-3.5 w-3.5" />}
            Fetch
          </button>
        </div>
      )}

      {error && (
        <div className="mt-3 flex items-center gap-2 rounded-md bg-[var(--status-error)]/10 px-3 py-2 text-xs text-[var(--status-error)]">
          <AlertCircle className="h-3.5 w-3.5" />
          {error}
        </div>
      )}

      {success && (
        <div className="mt-3 flex items-center gap-2 rounded-md bg-[var(--status-online)]/10 px-3 py-2 text-xs text-[var(--status-online)]">
          <Check className="h-3.5 w-3.5" />
          WASM loaded successfully
        </div>
      )}

      <button
        onClick={onCancel}
        className="mt-4 w-full rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground hover:bg-secondary/80"
      >
        Cancel
      </button>
    </div>
  )
}
