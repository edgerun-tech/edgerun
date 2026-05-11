"use client"

import * as React from "react"
import {
  Archive,
  Boxes,
  ChevronDown,
  ChevronRight,
  Code2,
  Database,
  File,
  FileAudio,
  FileArchive,
  FileCode2,
  FileCog,
  FileImage,
  FileJson,
  FileLock2,
  FileSpreadsheet,
  FileText,
  FileVideo,
  Folder,
  FolderGit2,
  FolderOpen,
  GitBranch,
  Inbox,
  KeyRound,
  LayoutDashboard,
  Mail,
  Network,
  Package,
  Route,
  Server,
  Settings,
  TableProperties,
  Terminal,
  Pencil,
  Trash2,
  Vault,
  Workflow,
} from "lucide-react"
import { useStore } from "@nanostores/react"
import { useForm } from "react-hook-form"
import { formatBytes } from "@/lib/format"

import componentInventory from "@/COMPONENT_INVENTORY.json"
import { AgentAudioVisualizerAura } from "@/components/agents-ui/agent-audio-visualizer-aura"
import { AgentAudioVisualizerBar } from "@/components/agents-ui/agent-audio-visualizer-bar"
import { AgentAudioVisualizerWave } from "@/components/agents-ui/agent-audio-visualizer-wave"
import { ReactShaderToy } from "@/components/agents-ui/react-shader-toy"
import { AlertCenter } from "@/components/AlertCenter"
import { CapabilityList } from "@/components/CapabilityList"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import {
  DependencyGraph,
  updateDependencyGraph,
} from "@/components/DependencyGraph"
import { PipelineProgress } from "@/components/PipelineProgress"
import { CodelyzerCodeWidget, CodelyzerNetworkPanel, NetworkConnectionsWidget } from "@/components/sections/codelyzer-network"
import { FinancesOverviewWidget } from "@/components/sections/finance-overviews"
import { TestStatusPanel } from "@/components/tests/TestStatusPanel"
import { TokenUsagePanel } from "@/components/tokens/TokenUsagePanel"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Modal, ModalBody, ModalContent, ModalFooter, ModalTrigger } from "@/components/ui/animated-modal"
import { AspectRatio } from "@/components/ui/aspect-ratio"
import { Avatar, AvatarFallback, AvatarGroup, AvatarGroupCount } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb"
import { Button } from "@/components/ui/button"
import { ButtonGroup, ButtonGroupSeparator, ButtonGroupText } from "@/components/ui/button-group"
import { Calendar } from "@/components/ui/calendar"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Carousel,
  CarouselContent,
  CarouselItem,
  CarouselNext,
  CarouselPrevious,
} from "@/components/ui/carousel"
import { Checkbox } from "@/components/ui/checkbox"
import { CodeBlock } from "@/components/ui/code-block"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import {
  Combobox,
  ComboboxContent,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from "@/components/ui/combobox"
import {
  Command,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "@/components/ui/command"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"
import { DirectionProvider } from "@/components/ui/direction"
import { Drawer, DrawerContent, DrawerDescription, DrawerHeader, DrawerTitle, DrawerTrigger } from "@/components/ui/drawer"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Field, FieldContent, FieldDescription, FieldGroup, FieldLabel, FieldTitle } from "@/components/ui/field"
import {
  FloatingDock,
  type FloatingDockContext,
  type FloatingDockItem,
} from "@/components/ui/floating-dock"
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel } from "@/components/ui/form"
import { GlowingEffect } from "@/components/ui/glowing-effect"
import { GlowingStarsBackgroundCard, GlowingStarsDescription, GlowingStarsTitle } from "@/components/ui/glowing-stars"
import { GridView } from "@/components/ui/grid-view"
import { HoverCard, HoverCardContent, HoverCardTrigger } from "@/components/ui/hover-card"
import { Input } from "@/components/ui/input"
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupText } from "@/components/ui/input-group"
import { InputOTP, InputOTPGroup, InputOTPSlot } from "@/components/ui/input-otp"
import { Item, ItemContent, ItemDescription, ItemGroup, ItemMedia, ItemTitle } from "@/components/ui/item"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Label } from "@/components/ui/label"
import { LayoutGrid } from "@/components/ui/layout-grid"
import {
  Menubar,
  MenubarContent,
  MenubarItem,
  MenubarMenu,
  MenubarTrigger,
} from "@/components/ui/menubar"
import { Button as MovingBorderButton } from "@/components/ui/moving-border"
import { MultiStepLoader } from "@/components/ui/multi-step-loader"
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select"
import {
  NavigationMenu,
  NavigationMenuContent,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
  NavigationMenuTrigger,
} from "@/components/ui/navigation-menu"
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination"
import { Popover, PopoverContent, PopoverDescription, PopoverHeader, PopoverTitle, PopoverTrigger } from "@/components/ui/popover"
import { Progress } from "@/components/ui/progress"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet"
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar"
import { Skeleton } from "@/components/ui/skeleton"
import { Slider } from "@/components/ui/slider"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { Toggle } from "@/components/ui/toggle"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import WorldMap from "@/components/ui/world-map"
import { WasmInstaller } from "@/components/wasm-installer"
import { WorkspacePromptInput } from "@/components/workspace/workspace-prompt-input"
import { WorkspaceStatusPanel } from "@/components/workspace/workspace-status-panel"
import { XrayCommandSurface } from "@/features/xray/XrayCommandSurface"
import { XrayInspector } from "@/features/xray/XrayInspector"
import { XrayLegendOverlay, XrayMouseHelpOverlay } from "@/features/xray/XrayOverlayWidgets"
import { XrayViewport } from "@/features/xray/XrayViewport"
import { XrayWorkspace } from "@/features/xray/XrayWorkspace"
import { codebaseStore, scanCodebase } from "@/stores/codebase-context"
import {
  compressEntry,
  deleteEntry,
  fileSystemStore,
  isFileSystemAccessSupported,
  openDirectory,
  openDirectoryFromFileList,
  openFile,
  readFileContent,
  renameEntry,
  toggleDirectory,
  type FileEntry,
  type GitInfo,
} from "@/stores/file-system-store"
import {
  aggregateEntries,
  buildWorkspaceDependencyGraph,
  codeHotspots,
  compactChartData,
  extractFunctions,
  extractImports,
  fileTypeBucket,
  importMatchesPath,
  managementSuggestions,
  segmentColor,
  type FileCodeMeta,
} from "@/workspace/workspace-index"

type ComponentInventoryEntry = {
  path: string
  group: string
  exports: string[]
}

type FileSortMode = "name" | "type" | "size" | "modified"
type FileShowMode = "all" | "files" | "folders"
type FileGroupMode = "none" | "kind" | "type" | "folder"
type FileViewMode = "chart" | "grid" | "table"

type AnalyzerProgress = {
  active: boolean
  phase: "idle" | "compiling" | "counting" | "analyzing" | "done" | "error"
  completed: number
  total: number
  label: string
}

const inventoryEntries = componentInventory.entries as ComponentInventoryEntry[]

const sideEffectSafeEntries = inventoryEntries.filter((entry) => {
  if (entry.group !== "components/ui") return false
  if (entry.path.includes("floating-dock")) return false
  if (entry.path.includes("toaster") || entry.path.includes("sonner")) return false
  return true
})

const carouselEntries = inventoryEntries.filter((entry) => {
  if (entry.path.includes("floating-dock")) return false
  if (entry.path.includes("toaster") || entry.path.includes("sonner")) return false
  if (entry.group === "components/ui") return true
  if (entry.group === "sections") return true
  if (entry.group === "components/other") return true
  if (entry.group.startsWith("features/")) return true
  return false
})

type CollectionId = "primitives" | "sections" | "candidates" | "xray" | "all"

const componentCollections: Array<{
  id: CollectionId
  label: string
  description: string
  filter: (entry: ComponentInventoryEntry) => boolean
}> = [
  {
    id: "sections",
    label: "Sections",
    description: "Reusable content blocks and dashboards.",
    filter: (entry) => entry.group === "sections",
  },
  {
    id: "candidates",
    label: "Candidates",
    description: "Non-app component surfaces worth evaluating.",
    filter: (entry) => entry.group === "components/other",
  },
  {
    id: "xray",
    label: "Xray",
    description: "Graph and visual inspection surfaces.",
    filter: (entry) => entry.group.startsWith("features/xray"),
  },
  {
    id: "primitives",
    label: "Primitives",
    description: "Side-effect-safe UI building blocks.",
    filter: (entry) => entry.group === "components/ui",
  },
  {
    id: "all",
    label: "All Safe",
    description: "Everything except dock/toast globals and full apps.",
    filter: (entry) => carouselEntries.includes(entry),
  },
]

function collectionEntries(collectionId: CollectionId) {
  const collection = componentCollections.find((item) => item.id === collectionId)
  if (!collection) return carouselEntries
  return inventoryEntries
    .filter((entry) => !entry.path.includes("floating-dock"))
    .filter((entry) => !entry.path.includes("toaster") && !entry.path.includes("sonner"))
    .filter((entry) => entry.group !== "apps")
    .filter(collection.filter)
}

function componentTitle(entry: ComponentInventoryEntry) {
  const namedExport = entry.exports.find(
    (name) => !name.startsWith("type ") && !name.startsWith("interface ") && name !== "default export"
  )
  if (namedExport) return namedExport

  const file = entry.path.split("/").pop() ?? entry.path
  return file.replace(/\.(tsx|ts|jsx|js)$/, "")
}

function groupIcon(group: string) {
  if (group === "components/ui") return Boxes
  if (group === "apps" || group === "os-shell") return LayoutDashboard
  if (group === "sections") return TableProperties
  if (group.startsWith("features/")) return Workflow
  if (group === "previews") return Code2
  return Database
}

function DockIcon({ children }: { children: React.ReactNode }) {
  return (
    <span className="flex h-full w-full items-center justify-center rounded-full text-white">
      {children}
    </span>
  )
}

function createDockItems(onOpenWorkspace: () => void, workspaceName?: string | null): FloatingDockItem[] {
  return [
    {
      title: workspaceName ? "Workspace" : "Open Folder",
      subtitle: workspaceName ?? "local folder",
      icon: (
        <DockIcon>
          <FolderOpen className="h-5 w-5" />
        </DockIcon>
      ),
      onClick: onOpenWorkspace,
      kind: "trigger",
    },
    {
      title: "Dashboard",
      subtitle: "canvas",
      icon: (
        <DockIcon>
          <LayoutDashboard className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Components",
      subtitle: "catalog",
      icon: (
        <DockIcon>
          <Boxes className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Data",
      subtitle: "queries",
      icon: (
        <DockIcon>
          <Database className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Tables",
      subtitle: "records",
      icon: (
        <DockIcon>
          <TableProperties className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Graphs",
      subtitle: "relations",
      icon: (
        <DockIcon>
          <Workflow className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Messages",
      subtitle: "transport",
      icon: (
        <DockIcon>
          <Inbox className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Mail",
      subtitle: "accounts",
      icon: (
        <DockIcon>
          <Mail className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Routes",
      subtitle: "policy",
      icon: (
        <DockIcon>
          <Route className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Secrets",
      subtitle: "vault",
      icon: (
        <DockIcon>
          <Vault className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
    {
      title: "Code",
      subtitle: "editor",
      icon: (
        <DockIcon>
          <Code2 className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "app",
    },
  ]
}

const dockContext: FloatingDockContext = {
  mode: "apps",
  items: [
    {
      title: "Node",
      subtitle: "ken-main-vps",
      icon: (
        <DockIcon>
          <Server className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "trigger",
    },
    {
      title: "Network",
      subtitle: "sync",
      icon: (
        <DockIcon>
          <Network className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "trigger",
    },
    {
      title: "Key",
      subtitle: "unlocked",
      icon: (
        <DockIcon>
          <KeyRound className="h-5 w-5" />
        </DockIcon>
      ),
      kind: "trigger",
    },
  ],
}

const layoutGridCards = [
  {
    id: 1,
    content: <div className="text-xs font-medium text-white">Storage</div>,
    className: "col-span-1",
    thumbnail:
      "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 320 220'%3E%3Crect width='320' height='220' fill='%230f172a'/%3E%3Ccircle cx='86' cy='70' r='34' fill='%230ea5e9'/%3E%3Crect x='42' y='125' width='236' height='18' rx='9' fill='%238b5cf6'/%3E%3Crect x='42' y='156' width='170' height='14' rx='7' fill='%2322c55e'/%3E%3C/svg%3E",
  },
  {
    id: 2,
    content: <div className="text-xs font-medium text-white">Messages</div>,
    className: "col-span-1",
    thumbnail:
      "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 320 220'%3E%3Crect width='320' height='220' fill='%23111827'/%3E%3Cpath d='M52 62h216v92H92l-40 34z' fill='%2314b8a6'/%3E%3Crect x='86' y='88' width='140' height='12' rx='6' fill='white' opacity='.75'/%3E%3Crect x='86' y='116' width='92' height='12' rx='6' fill='white' opacity='.45'/%3E%3C/svg%3E",
  },
]

function FormFixture() {
  const form = useForm<{ endpoint: string }>({
    defaultValues: { endpoint: "node.local" },
  })

  return (
    <Form {...form}>
      <form className="w-full">
        <FormField
          control={form.control}
          name="endpoint"
          render={({ field }) => (
            <FormItem>
              <FormLabel className="text-xs">Endpoint</FormLabel>
              <FormControl>
                <Input className="h-8" {...field} />
              </FormControl>
              <FormDescription className="text-[10px]">Controlled form field</FormDescription>
            </FormItem>
          )}
        />
      </form>
    </Form>
  )
}

function HookPreview({ name }: { name: string }) {
  return (
    <span className="flex flex-col items-center gap-2 text-center">
      <Badge variant="outline" className="rounded-md">hook</Badge>
      <span className="font-mono text-[10px] text-muted-foreground">{name}</span>
    </span>
  )
}

function PanelPreviewFrame({ children }: { children: React.ReactNode }) {
  return (
    <div className="max-h-36 w-full overflow-auto rounded-md border bg-background/80 p-3 text-left">
      {children}
    </div>
  )
}

function fileIcon(entry: FileEntry) {
  if (entry.kind === "directory") {
    if (entry.name === ".git") return FolderGit2
    if (entry.expanded) return FolderOpen
    return Folder
  }
  const ext = entry.name.split(".").pop()?.toLowerCase()
  const lower = entry.name.toLowerCase()
  if (["package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lock", "bun.lockb", "cargo.toml", "cargo.lock", "go.mod", "go.sum", "requirements.txt", "pyproject.toml"].includes(lower)) return Package
  if (["dockerfile", "containerfile", "makefile", "justfile"].includes(lower) || ["sh", "bash", "zsh", "fish", "ps1"].includes(ext ?? "")) return Terminal
  if ([".env", ".env.local", ".env.production"].includes(lower) || ["env", "pem", "key", "crt", "cer"].includes(ext ?? "") || lower.includes("secret")) return FileLock2
  if (lower.startsWith(".git") || lower.startsWith(".eslint") || lower.startsWith(".prettier") || lower.startsWith("tsconfig") || lower.startsWith("vite.config") || lower.startsWith("next.config") || ["config", "conf", "ini"].includes(ext ?? "") || lower.includes("config")) return Settings
  if (["ts", "tsx", "js", "jsx", "rs", "go", "py", "css", "scss", "sass", "html", "vue", "svelte", "c", "cpp", "h", "hpp", "java", "kt", "swift"].includes(ext ?? "")) return FileCode2
  if (["json", "toml", "yaml", "yml", "xml", "lock"].includes(ext ?? "")) return FileJson
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext ?? "")) return FileImage
  if (["mp3", "wav", "ogg", "flac"].includes(ext ?? "")) return FileAudio
  if (["mp4", "mov", "webm", "mkv"].includes(ext ?? "")) return FileVideo
  if (["csv", "tsv", "xls", "xlsx"].includes(ext ?? "")) return FileSpreadsheet
  if (["zip", "gz", "tar", "rar", "7z"].includes(ext ?? "")) return FileArchive
  if (["md", "txt", "log", "rst"].includes(ext ?? "")) return FileText
  if (["service", "timer", "desktop"].includes(ext ?? "")) return FileCog
  return File
}

function fileIconClass(entry: FileEntry) {
  if (entry.kind === "directory") return entry.name === ".git" ? "text-orange-400" : "text-amber-300"
  const ext = entry.name.split(".").pop()?.toLowerCase()
  const lower = entry.name.toLowerCase()
  if (["package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lock", "bun.lockb"].includes(lower)) return "text-lime-300"
  if (["cargo.toml", "cargo.lock"].includes(lower) || ext === "rs") return "text-orange-400"
  if (["go.mod", "go.sum"].includes(lower) || ext === "go") return "text-cyan-300"
  if (["requirements.txt", "pyproject.toml"].includes(lower) || ext === "py") return "text-yellow-300"
  if (["dockerfile", "containerfile"].includes(lower)) return "text-sky-300"
  if (["makefile", "justfile"].includes(lower) || ["sh", "bash", "zsh", "fish", "ps1"].includes(ext ?? "")) return "text-lime-300"
  if ([".env", ".env.local", ".env.production"].includes(lower) || ["env", "pem", "key", "crt", "cer"].includes(ext ?? "") || lower.includes("secret")) return "text-red-300"
  if (lower.startsWith(".git")) return "text-orange-300"
  if (lower.startsWith(".eslint") || lower.startsWith(".prettier") || lower.startsWith("tsconfig") || lower.startsWith("vite.config") || lower.startsWith("next.config") || ["config", "conf", "ini"].includes(ext ?? "") || lower.includes("config")) return "text-cyan-300"
  if (["ts", "tsx"].includes(ext ?? "")) return "text-sky-300"
  if (["js", "jsx"].includes(ext ?? "")) return "text-yellow-300"
  if (["css", "scss", "sass"].includes(ext ?? "")) return "text-violet-300"
  if (ext === "html") return "text-orange-300"
  if (["vue", "svelte"].includes(ext ?? "")) return "text-emerald-300"
  if (["c", "cpp", "h", "hpp", "java", "kt", "swift"].includes(ext ?? "")) return "text-blue-300"
  if (["json", "toml", "yaml", "yml", "xml", "lock"].includes(ext ?? "")) return "text-emerald-300"
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext ?? "")) return "text-fuchsia-300"
  if (["mp3", "wav", "ogg", "flac"].includes(ext ?? "")) return "text-pink-300"
  if (["mp4", "mov", "webm", "mkv"].includes(ext ?? "")) return "text-violet-300"
  if (["csv", "tsv", "xls", "xlsx"].includes(ext ?? "")) return "text-green-300"
  if (["zip", "gz", "tar", "rar", "7z"].includes(ext ?? "")) return "text-purple-300"
  if (["md", "txt", "log", "rst"].includes(ext ?? "")) return "text-zinc-300"
  return "text-muted-foreground"
}

function visibleTreeEntries(entries: FileEntry[], expandedDirs: string[]) {
  const expanded = new Set(expandedDirs)
  return entries.filter((entry) => {
    const parents = entry.path.split("/").slice(0, -1)
    let current = ""
    for (const parent of parents) {
      current = current ? `${current}/${parent}` : parent
      if (!expanded.has(current)) return false
    }
    return true
  })
}

function directorySizeMap(entries: FileEntry[]) {
  const sizes = new Map<string, number>()
  for (const entry of entries) {
    if (entry.kind !== "file") continue
    const size = entry.size ?? 0
    const parts = entry.path.split("/")
    parts.pop()
    let current = ""
    for (const part of parts) {
      current = current ? `${current}/${part}` : part
      sizes.set(current, (sizes.get(current) ?? 0) + size)
    }
  }
  return sizes
}

function childCountMap(entries: FileEntry[]) {
  const counts = new Map<string, number>()
  for (const entry of entries) {
    const parent = entry.parentPath
    if (parent) counts.set(parent, (counts.get(parent) ?? 0) + 1)
  }
  return counts
}

function formatDate(timestamp?: number) {
  if (!timestamp) return "-"
  return new Date(timestamp).toLocaleString()
}

function compactList(items: string[], max = 3) {
  if (items.length === 0) return "-"
  const head = items.slice(0, max).join(", ")
  return items.length > max ? `${head} +${items.length - max}` : head
}

function orderFileEntries(
  entries: FileEntry[],
  sortMode: FileSortMode,
  showMode: FileShowMode,
  groupMode: FileGroupMode,
) {
  const visible = entries.filter((entry) => {
    if (showMode === "files") return entry.kind === "file"
    if (showMode === "folders") return entry.kind === "directory"
    return true
  })

  const groupValue = (entry: FileEntry) => {
    if (groupMode === "kind") return entry.kind
    if (groupMode === "type") return fileTypeBucket(entry)
    if (groupMode === "folder") return entry.parentPath || "root"
    return ""
  }

  return [...visible].sort((a, b) => {
    const groupCompare = groupValue(a).localeCompare(groupValue(b))
    if (groupCompare !== 0) return groupCompare

    if (sortMode === "type") {
      const typeCompare = fileTypeBucket(a).localeCompare(fileTypeBucket(b))
      if (typeCompare !== 0) return typeCompare
    }
    if (sortMode === "size") {
      const sizeCompare = (b.size ?? 0) - (a.size ?? 0)
      if (sizeCompare !== 0) return sizeCompare
    }
    if (sortMode === "modified") {
      const modifiedCompare = (b.modified ?? 0) - (a.modified ?? 0)
      if (modifiedCompare !== 0) return modifiedCompare
    }
    if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1
    return a.name.localeCompare(b.name)
  })
}

function FileGridPreview({
  entry,
  entries,
}: {
  entry: FileEntry
  entries: FileEntry[]
}) {
  const [snippet, setSnippet] = React.useState<string | null>(null)
  const [loaded, setLoaded] = React.useState(false)
  const Icon = fileIcon(entry)
  const folderSizes = React.useMemo(() => directorySizeMap(entries), [entries])
  const childCounts = React.useMemo(() => childCountMap(entries), [entries])
  const size = entry.kind === "directory" ? folderSizes.get(entry.path) : entry.size
  const extension = entry.kind === "file" ? entry.name.split(".").pop()?.toLowerCase() : null
  const canReadPreview =
    entry.kind === "file" &&
    !entry.binary &&
    (entry.size ?? 0) <= 96 * 1024 &&
    !["png", "jpg", "jpeg", "gif", "webp", "ico", "wasm", "pdf", "zip", "gz"].includes(extension ?? "")

  async function loadPreview() {
    if (loaded || !canReadPreview) return
    setLoaded(true)
    const content = await readFileContent(entry.path).catch(() => null)
    if (content) setSnippet(content.slice(0, 420))
  }

  return (
    <div
      className="flex size-full flex-col"
      onMouseEnter={() => void loadPreview()}
      onFocus={() => void loadPreview()}
    >
      <div className="flex min-h-0 flex-1 items-center justify-center overflow-hidden rounded-md border border-white/10 bg-black/15 p-3">
        {entry.kind === "directory" ? (
          <div className="flex flex-col items-center gap-2">
            <FolderOpen className="size-12 text-amber-300" />
            <Badge variant="outline" className="rounded-md text-[10px]">
              {childCounts.get(entry.path) ?? 0} items
            </Badge>
          </div>
        ) : snippet ? (
          <pre className="size-full overflow-hidden whitespace-pre-wrap break-words text-left font-mono text-[9px] leading-snug text-muted-foreground">
            {snippet}
          </pre>
        ) : (
          <div className="flex flex-col items-center gap-2">
            <Icon className={`size-12 ${fileIconClass(entry)}`} />
            <span className="rounded-md border border-white/10 bg-white/5 px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
              {extension || "file"}
            </span>
          </div>
        )}
      </div>
      <div className="mt-3 min-w-0 text-center">
        <div className="truncate text-sm font-medium">{entry.name}</div>
        <div className="mt-1 truncate text-xs text-muted-foreground">{entry.parentPath || "root"}</div>
        <div className="mt-1 truncate font-mono text-[10px] text-muted-foreground/80">{formatBytes(size)}</div>
      </div>
    </div>
  )
}

function WorkspaceTableView({
  entries,
  selectedPaths,
  onSelect,
  metadata,
}: {
  entries: FileEntry[]
  selectedPaths: string[]
  onSelect: (entry: FileEntry, event: React.MouseEvent, visibleEntries: FileEntry[]) => void
  metadata: Record<string, FileCodeMeta>
}) {
  return (
    <div className="h-full overflow-auto px-6 pb-32 pt-6 sm:px-8">
      <div className="min-w-[1280px] rounded-md border border-white/10 bg-background/72 text-xs shadow-2xl backdrop-blur-xl">
        <Table>
          <TableHeader className="sticky top-0 z-10 bg-background/90 backdrop-blur-xl">
            <TableRow>
              <TableHead className="w-[280px]">Full path</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Imports</TableHead>
              <TableHead>Imported by</TableHead>
              <TableHead className="text-right">LOC</TableHead>
              <TableHead>Functions</TableHead>
              <TableHead>Created</TableHead>
              <TableHead>Modified</TableHead>
              <TableHead className="text-right">Size</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {entries.map((entry) => {
              const Icon = fileIcon(entry)
              const meta = metadata[entry.path]
              const selected = selectedPaths.includes(entry.path)

              return (
                <ContextMenu key={entry.path}>
                  <ContextMenuTrigger asChild>
                    <TableRow
                      className={selected ? "bg-primary/20" : ""}
                      onClick={(event) => onSelect(entry, event, entries)}
                      onDoubleClick={() => {
                        if (entry.kind === "directory") void toggleDirectory(entry.path)
                        else void openFile(entry.path)
                      }}
                    >
                      <TableCell className="max-w-[360px]">
                        <div className="flex min-w-0 items-center gap-2">
                          <Icon className={`size-4 shrink-0 ${fileIconClass(entry)}`} />
                          <span className="truncate font-mono" title={entry.path}>{entry.path}</span>
                        </div>
                      </TableCell>
                      <TableCell>{fileTypeBucket(entry)}</TableCell>
                      <TableCell className="max-w-[220px] truncate" title={meta?.imports.join("\n")}>{compactList(meta?.imports ?? [])}</TableCell>
                      <TableCell className="max-w-[220px] truncate" title={meta?.importedBy.join("\n")}>{compactList(meta?.importedBy ?? [])}</TableCell>
                      <TableCell className="text-right font-mono">{meta?.linesOfCode ?? "-"}</TableCell>
                      <TableCell className="max-w-[240px] truncate" title={meta?.functions.join("\n")}>{compactList(meta?.functions ?? [])}</TableCell>
                      <TableCell>-</TableCell>
                      <TableCell>{formatDate(entry.modified)}</TableCell>
                      <TableCell className="text-right font-mono">{formatBytes(entry.size)}</TableCell>
                    </TableRow>
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    <ContextMenuItem onClick={() => void renameEntry(entry.path, window.prompt("Rename to", entry.name) || entry.name)}>
                      <Pencil className="mr-2 size-3.5" /> Rename
                    </ContextMenuItem>
                    <ContextMenuItem onClick={() => void compressEntry(entry.path)}>
                      <Archive className="mr-2 size-3.5" /> Compress
                    </ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem variant="destructive" onClick={() => void deleteEntry(entry.path)}>
                      <Trash2 className="mr-2 size-3.5" /> Delete
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              )
            })}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}

function DonutChart({
  data,
  valueKey,
  title,
}: {
  data: Array<{ label: string; count: number; size: number }>
  valueKey: "count" | "size"
  title: string
}) {
  const chartData = compactChartData(data, valueKey === "size" ? 7 : 9)
  const total = chartData.reduce((sum, item) => sum + item[valueKey], 0)
  let offset = 25

  return (
    <Card className="min-h-0 gap-3 bg-background/72">
      <CardHeader className="pb-0">
        <CardTitle className="text-sm">{title}</CardTitle>
        <CardDescription>{valueKey === "size" ? formatBytes(total) : `${total} entries`}</CardDescription>
      </CardHeader>
      <CardContent className="grid min-h-0 grid-cols-1 gap-4 2xl:grid-cols-[150px_1fr]">
        <svg viewBox="0 0 120 120" className="mx-auto size-36">
          <circle cx="60" cy="60" r="44" fill="none" stroke="oklch(1 0 0 / 0.08)" strokeWidth="18" />
          {total > 0 && chartData.map((item, index) => {
            const value = item[valueKey]
            const length = (value / total) * 276.46
            const currentOffset = offset
            offset -= length
            return (
              <circle
                key={item.label}
                cx="60"
                cy="60"
                r="44"
                fill="none"
                stroke={segmentColor(index)}
                strokeWidth="18"
                strokeDasharray={`${length} 276.46`}
                strokeDashoffset={currentOffset}
                strokeLinecap="butt"
                transform="rotate(-90 60 60)"
              />
            )
          })}
          <text x="60" y="56" textAnchor="middle" className="fill-foreground text-[13px] font-semibold">
            {data.length}
          </text>
          <text x="60" y="72" textAnchor="middle" className="fill-muted-foreground text-[8px]">
            groups
          </text>
        </svg>
        <div className="min-w-0 space-y-2 overflow-auto py-1 2xl:max-h-44">
          {chartData.map((item, index) => (
            <div key={item.label} className="grid grid-cols-[10px_1fr_auto] items-center gap-2 text-xs">
              <span className="size-2.5 rounded-full" style={{ backgroundColor: segmentColor(index) }} />
              <span className="truncate">{item.label}</span>
              <span className="font-mono text-muted-foreground">
                {valueKey === "size" ? formatBytes(item.size) : item.count}
              </span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}

function WorkspaceMetric({
  label,
  value,
  detail,
  icon: Icon,
}: {
  label: string
  value: React.ReactNode
  detail?: string
  icon: React.ComponentType<{ className?: string }>
}) {
  return (
    <div className="min-w-0 rounded-md border border-white/10 bg-white/[0.035] p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="truncate text-[11px] font-medium uppercase tracking-normal text-muted-foreground">{label}</div>
        <Icon className="size-4 shrink-0 text-muted-foreground" />
      </div>
      <div className="truncate text-xl font-semibold tabular-nums">{value}</div>
      {detail ? <div className="mt-1 truncate text-xs text-muted-foreground">{detail}</div> : null}
    </div>
  )
}

function WorkspaceDistributionBars({
  title,
  data,
  valueKey,
}: {
  title: string
  data: Array<{ label: string; count: number; size: number }>
  valueKey: "count" | "size"
}) {
  const rows = compactChartData(data, 6)
  const total = rows.reduce((sum, item) => sum + item[valueKey], 0)

  return (
    <div className="rounded-md border border-white/10 bg-white/[0.03] p-4">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div>
          <div className="text-sm font-semibold">{title}</div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            {valueKey === "size" ? formatBytes(total) : `${total} entries`}
          </div>
        </div>
        <TableProperties className="size-4 text-muted-foreground" />
      </div>
      <div className="space-y-3">
        {rows.map((item, index) => {
          const value = item[valueKey]
          const percent = total > 0 ? Math.max(3, Math.round((value / total) * 100)) : 0
          return (
            <div key={item.label} className="grid grid-cols-[minmax(0,1fr)_auto] gap-x-3 gap-y-1 text-xs">
              <div className="truncate font-medium">{item.label}</div>
              <div className="font-mono text-muted-foreground">
                {valueKey === "size" ? formatBytes(item.size) : item.count}
              </div>
              <div className="col-span-2 h-2 overflow-hidden rounded-full bg-white/10">
                <div
                  className="h-full rounded-full"
                  style={{ width: `${percent}%`, backgroundColor: segmentColor(index) }}
                />
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

function WorkspaceInsight({
  title,
  detail,
  tone,
}: {
  title: string
  detail: string
  tone: string
}) {
  return (
    <div className={`rounded-md border p-3 ${tone}`}>
      <div className="text-sm font-medium">{title}</div>
      <div className="mt-1 text-xs leading-relaxed text-muted-foreground">{detail}</div>
    </div>
  )
}

function WorkspaceCompositionView({
  entries,
  metadata,
  isGitRepo,
  gitInfo,
}: {
  entries: FileEntry[]
  metadata: Record<string, FileCodeMeta>
  isGitRepo: boolean
  gitInfo: GitInfo | null
}) {
  const aggregate = React.useMemo(() => aggregateEntries(entries, metadata), [entries, metadata])
  const hotspots = React.useMemo(() => codeHotspots(entries, metadata), [entries, metadata])
  const suggestions = React.useMemo(() => managementSuggestions(entries, metadata, isGitRepo), [entries, metadata, isGitRepo])
  const files = entries.filter((entry) => entry.kind === "file").length
  const folders = entries.filter((entry) => entry.kind === "directory").length
  const imports = Object.values(metadata).reduce((sum, item) => sum + item.imports.length, 0)
  const importedBy = Object.values(metadata).reduce((sum, item) => sum + item.importedBy.length, 0)
  const measuredPercent = files > 0 ? Math.round((aggregate.measuredCodeFiles / files) * 100) : 0
  const topCategory = aggregate.byCategory[0]
  const topType = aggregate.byType.find((item) => item.label !== "folder") ?? aggregate.byType[0]
  const gitLabel = isGitRepo ? (gitInfo?.branch ?? gitInfo?.head ?? "repository") : "not detected"
  const healthItems = [
    {
      label: "Analysis coverage",
      value: `${measuredPercent}%`,
      active: measuredPercent >= 70,
    },
    {
      label: "Git context",
      value: gitLabel,
      active: isGitRepo,
    },
    {
      label: "Dependency signal",
      value: imports > 0 || importedBy > 0 ? `${imports + importedBy} links` : "not mapped",
      active: imports > 0 || importedBy > 0,
    },
  ]

  return (
    <div className="h-full overflow-auto px-4 pb-32 pt-4 sm:px-6">
      <div className="grid w-full max-w-none grid-cols-1 gap-4 2xl:grid-cols-12">
        <section className="rounded-md border border-white/10 bg-background/72 p-4 shadow-2xl backdrop-blur-xl 2xl:col-span-12">
          <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_440px] xl:items-end">
            <div className="min-w-0">
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <Badge variant="outline" className="rounded-md">Workspace</Badge>
                <Badge variant="outline" className="rounded-md">{files} files</Badge>
                <Badge variant="outline" className="rounded-md">{folders} folders</Badge>
              </div>
              <h2 className="truncate text-2xl font-semibold tracking-normal">Workspace overview</h2>
              <p className="mt-1 max-w-3xl text-sm leading-relaxed text-muted-foreground">
                The current folder is {formatBytes(aggregate.totalSize)} across {entries.length} indexed entries.
                {topCategory ? ` Most of the footprint is ${topCategory.label.toLowerCase()}.` : ""}
                {topType ? ` The most common file type is ${topType.label}.` : ""}
              </p>
            </div>
            <div className="grid gap-2 sm:grid-cols-3">
              {healthItems.map((item) => (
                <div key={item.label} className="rounded-md border border-white/10 bg-white/[0.03] p-3">
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <span className={`size-2 rounded-full ${item.active ? "bg-emerald-400" : "bg-amber-400"}`} />
                    <span className="truncate">{item.label}</span>
                  </div>
                  <div className="mt-2 truncate font-mono text-sm">{item.value}</div>
                </div>
              ))}
            </div>
          </div>
        </section>

        <div className="grid grid-cols-2 gap-3 2xl:col-span-12 lg:grid-cols-4">
          <WorkspaceMetric label="Total size" value={formatBytes(aggregate.totalSize)} detail={`${entries.length} indexed entries`} icon={Database} />
          <WorkspaceMetric label="Code files" value={aggregate.measuredCodeFiles || "-"} detail={`${measuredPercent}% metadata coverage`} icon={Code2} />
          <WorkspaceMetric label="Lines" value={aggregate.totalLoc || "-"} detail={`${aggregate.totalFunctions || 0} functions found`} icon={FileCode2} />
          <WorkspaceMetric label="References" value={imports + importedBy || "-"} detail={`${imports || 0} imports, ${importedBy || 0} inbound`} icon={Network} />
        </div>

        <div className="grid grid-cols-1 gap-4 2xl:col-span-8 xl:grid-cols-2">
          <WorkspaceDistributionBars title="Largest footprint" data={aggregate.byCategory} valueKey="size" />
          <WorkspaceDistributionBars title="Most common types" data={aggregate.byType} valueKey="count" />
        </div>

        <Card className="min-h-0 overflow-hidden bg-background/72 2xl:col-span-4">
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Network className="size-4 text-sky-400" />
              Dependency graph
            </CardTitle>
            <CardDescription>Mapped imports and package references from indexed code.</CardDescription>
          </CardHeader>
          <CardContent className="max-h-72 overflow-auto">
            <DependencyGraph />
          </CardContent>
        </Card>

        <Card className="bg-background/72 2xl:col-span-4">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Workflow className="size-4 text-emerald-400" />
              Next moves
            </CardTitle>
            <CardDescription>Concrete actions suggested by the current workspace state.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            {suggestions.map((item) => (
              <WorkspaceInsight key={item.title} title={item.title} detail={item.detail} tone={item.tone} />
            ))}
          </CardContent>
        </Card>

        <Card className="bg-background/72 2xl:col-span-8">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Code2 className="size-4 text-violet-400" />
              Code hotspots
            </CardTitle>
            <CardDescription>Files with the strongest mix of LOC, functions, imports, and references.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            {hotspots.length > 0 ? hotspots.map((item) => (
              <button
                key={item.entry.path}
                type="button"
                className="grid w-full grid-cols-1 gap-3 rounded-md border border-white/10 bg-white/[0.03] p-3 text-left transition-colors hover:border-primary/50 hover:bg-primary/10 md:grid-cols-[1fr_auto]"
                onClick={() => void openFile(item.entry.path)}
              >
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{item.entry.name}</div>
                  <div className="mt-1 truncate font-mono text-[11px] text-muted-foreground">{item.entry.path}</div>
                </div>
                <div className="grid grid-cols-4 gap-2 text-right font-mono text-[11px] text-muted-foreground">
                  <span>{item.loc} loc</span>
                  <span>{item.functions} fn</span>
                  <span>{item.imports} in</span>
                  <span>{item.importedBy} ref</span>
                </div>
              </button>
            )) : (
              <div className="rounded-md border border-white/10 p-4 text-sm text-muted-foreground">
                No code metadata yet. The worker will populate this after a supported folder handle is opened.
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-background/72 2xl:col-span-4">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-sm">
              <TableProperties className="size-4 text-amber-400" />
              Inventory signal
            </CardTitle>
            <CardDescription>What the current view can manage immediately.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Indexed entries</span>
              <span className="font-mono">{entries.length}</span>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Code metadata coverage</span>
              <span className="font-mono">{measuredPercent}%</span>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Largest category</span>
              <span className="max-w-40 truncate font-mono">{aggregate.byCategory[0]?.label ?? "-"}</span>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Git branch</span>
              <span className="max-w-40 truncate font-mono">{gitInfo?.branch ?? gitInfo?.head ?? "-"}</span>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Git remote</span>
              <span className="max-w-40 truncate font-mono" title={gitInfo?.remote ?? undefined}>{gitInfo?.remote ?? "-"}</span>
            </div>
            <Separator />
            <p className="text-xs leading-relaxed text-muted-foreground">
              Use the file tree for selection and operations, grid for previews, and table when the question is about paths, imports, and exact file metadata.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function WorkspaceProgress({ progress }: { progress: AnalyzerProgress }) {
  if (!progress.active) return null

  const percent = progress.total > 0
    ? Math.min(100, Math.round((progress.completed / progress.total) * 100))
    : 0

  return (
    <div className="pointer-events-none absolute left-1/2 top-5 z-50 w-[min(420px,calc(100vw-2rem))] -translate-x-1/2 rounded-md border border-white/10 bg-background/95 px-4 py-3 shadow-2xl backdrop-blur-xl">
      <Field className="gap-2">
        <FieldLabel htmlFor="workspace-analysis-progress" className="text-xs">
          <span className="capitalize">{progress.phase === "done" ? "Analysis complete" : `${progress.phase} workspace`}</span>
          <span className="ml-auto">{percent}%</span>
        </FieldLabel>
        <Progress value={percent} id="workspace-analysis-progress" />
        <div className="truncate text-[10px] text-muted-foreground">
          {progress.label}
        </div>
      </Field>
    </div>
  )
}

function WorkspaceFileTree({
  entries,
  expandedDirs,
  rootName,
  gitInfo,
  isGitRepo,
  visible,
  onToggleVisible,
  selectedPaths,
  onSelect,
  sortMode,
  showMode,
  groupMode,
  viewMode,
  onSortModeChange,
  onShowModeChange,
  onGroupModeChange,
  onViewModeChange,
}: {
  entries: FileEntry[]
  expandedDirs: string[]
  rootName: string
  gitInfo: GitInfo | null
  isGitRepo: boolean
  visible: boolean
  onToggleVisible: () => void
  selectedPaths: string[]
  onSelect: (entry: FileEntry, event: React.MouseEvent, visibleEntries: FileEntry[]) => void
  sortMode: FileSortMode
  showMode: FileShowMode
  groupMode: FileGroupMode
  viewMode: FileViewMode
  onSortModeChange: (mode: FileSortMode) => void
  onShowModeChange: (mode: FileShowMode) => void
  onGroupModeChange: (mode: FileGroupMode) => void
  onViewModeChange: (mode: FileViewMode) => void
}) {
  const folderSizes = React.useMemo(() => directorySizeMap(entries), [entries])
  const childCounts = React.useMemo(() => childCountMap(entries), [entries])
  const visibleEntries = React.useMemo(
    () => orderFileEntries(visibleTreeEntries(entries, expandedDirs), sortMode, showMode, groupMode),
    [entries, expandedDirs, sortMode, showMode, groupMode]
  )
  if (!visible || entries.length === 0) return null

  async function renamePath(entry: FileEntry) {
    const nextName = window.prompt("Rename to", entry.name)
    if (!nextName || nextName === entry.name) return
    await renameEntry(entry.path, nextName)
    await scanCodebase()
  }

  async function deletePath(entry: FileEntry) {
    const ok = window.confirm(`Delete ${entry.path}?`)
    if (!ok) return
    await deleteEntry(entry.path)
    await scanCodebase()
  }

  async function compressPath(entry: FileEntry) {
    await compressEntry(entry.path)
    await scanCodebase()
  }

  return (
    <aside className="h-full min-h-0 border-r border-white/10 bg-[oklch(0.085_0.014_255)] text-foreground shadow-xl">
      <div className="flex h-full flex-col">
        <div className="flex h-11 items-center justify-between gap-2 border-b border-white/10 px-2.5">
          <ContextMenu>
            <ContextMenuTrigger asChild>
              <div className="min-w-0 flex-1 cursor-context-menu rounded px-1 py-0.5">
                <div className="flex min-w-0 items-center gap-2">
                  {isGitRepo ? <GitBranch className="size-3.5 shrink-0 text-orange-400" /> : <FolderOpen className="size-3.5 shrink-0 text-amber-300" />}
                  <div className="truncate text-sm font-semibold">{rootName}</div>
                </div>
                <div className="truncate text-[10px] text-muted-foreground">
                  {visibleEntries.length}/{entries.length} entries · {selectedPaths.length} selected · {gitInfo?.branch ?? gitInfo?.head ?? (isGitRepo ? "git" : "no git")} · {viewMode}
                </div>
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onClick={() => onSortModeChange("name")}>Sort by name</ContextMenuItem>
              <ContextMenuItem onClick={() => onSortModeChange("type")}>Sort by type</ContextMenuItem>
              <ContextMenuItem onClick={() => onSortModeChange("size")}>Sort by size</ContextMenuItem>
              <ContextMenuItem onClick={() => onSortModeChange("modified")}>Sort by modified</ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onClick={() => onViewModeChange("chart")}>View as chart</ContextMenuItem>
              <ContextMenuItem onClick={() => onViewModeChange("grid")}>View as grid</ContextMenuItem>
              <ContextMenuItem onClick={() => onViewModeChange("table")}>View as table</ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onClick={() => onShowModeChange("all")}>Show all</ContextMenuItem>
              <ContextMenuItem onClick={() => onShowModeChange("files")}>Show files only</ContextMenuItem>
              <ContextMenuItem onClick={() => onShowModeChange("folders")}>Show folders only</ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onClick={() => onGroupModeChange("none")}>Group none</ContextMenuItem>
              <ContextMenuItem onClick={() => onGroupModeChange("kind")}>Group by kind</ContextMenuItem>
              <ContextMenuItem onClick={() => onGroupModeChange("type")}>Group by type</ContextMenuItem>
              <ContextMenuItem onClick={() => onGroupModeChange("folder")}>Group by parent folder</ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
          <Button type="button" size="sm" variant="ghost" className="h-7 px-2 text-xs" onClick={onToggleVisible}>
            Hide
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-auto px-1.5 py-1.5">
          {visibleEntries.map((entry) => {
            const expanded = expandedDirs.includes(entry.path)
            const Icon = fileIcon(entry)
            const size = entry.kind === "directory" ? folderSizes.get(entry.path) : entry.size
            const childCount = entry.kind === "directory" ? childCounts.get(entry.path) ?? 0 : null
            const selected = selectedPaths.includes(entry.path)

            return (
              <ContextMenu key={entry.path}>
                <ContextMenuTrigger asChild>
                  <button
                    type="button"
                    className={`flex h-6 w-full min-w-0 items-center gap-1 rounded px-1 text-left text-xs transition-colors hover:bg-white/10 ${selected ? "bg-primary/20 text-foreground ring-1 ring-primary/35" : "text-foreground/90"}`}
                    style={{ paddingLeft: `${3 + entry.depth * 12}px` }}
                    onClick={(event) => {
                      onSelect(entry, event, visibleEntries)
                      if (entry.kind === "directory") void toggleDirectory(entry.path)
                      else void openFile(entry.path)
                    }}
                    title={entry.path}
                  >
                    {entry.kind === "directory" ? (
                      expanded ? <ChevronDown className="size-3 shrink-0 text-muted-foreground" /> : <ChevronRight className="size-3 shrink-0 text-muted-foreground" />
                    ) : (
                      <span className="size-3 shrink-0" />
                    )}
                    <Icon className={`size-3.5 shrink-0 drop-shadow-sm ${fileIconClass(entry)}`} />
                    <span className="min-w-0 flex-1 truncate">{entry.name}</span>
                    {entry.kind === "directory" ? (
                      <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{childCount}</span>
                    ) : null}
                    <span className="shrink-0 font-mono text-[10px] text-muted-foreground/80">{formatBytes(size)}</span>
                  </button>
                </ContextMenuTrigger>
                <ContextMenuContent>
                  <ContextMenuItem onClick={() => void renamePath(entry)}>
                    <Pencil className="mr-2 size-3.5" /> Rename
                  </ContextMenuItem>
                  <ContextMenuItem onClick={() => void compressPath(entry)}>
                    <Archive className="mr-2 size-3.5" /> Compress
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuItem variant="destructive" onClick={() => void deletePath(entry)}>
                    <Trash2 className="mr-2 size-3.5" /> Delete
                  </ContextMenuItem>
                </ContextMenuContent>
              </ContextMenu>
            )
          })}
        </div>
      </div>
    </aside>
  )
}

function ComponentPreview({
  entry,
  title,
  icon: Icon,
}: {
  entry: ComponentInventoryEntry
  title: string
  icon: typeof Boxes
}) {
  const file = entry.path.split("/").pop()?.replace(/\.(tsx|ts|jsx|js)$/, "") ?? title

  switch (file) {
    case "agent-audio-visualizer-aura":
      return <AgentAudioVisualizerAura size="sm" state="thinking" color="#38bdf8" />
    case "agent-audio-visualizer-bar":
      return <AgentAudioVisualizerBar size="sm" state="speaking" color="#22c55e" />
    case "agent-audio-visualizer-wave":
      return <AgentAudioVisualizerWave size="sm" state="listening" color="#a78bfa" />
    case "accordion":
      return (
        <Accordion type="single" defaultValue="item">
          <AccordionItem value="item">
            <AccordionTrigger className="py-2 text-xs">Accordion</AccordionTrigger>
            <AccordionContent className="pb-2 text-[10px] text-muted-foreground">Content panel</AccordionContent>
          </AccordionItem>
        </Accordion>
      )
    case "alert-dialog":
      return (
        <AlertDialog>
          <AlertDialogTrigger asChild><Button size="sm" variant="outline">Confirm</Button></AlertDialogTrigger>
          <AlertDialogContent size="sm">
            <AlertDialogHeader>
              <AlertDialogTitle>Deploy node?</AlertDialogTitle>
              <AlertDialogDescription>Runs the selected action.</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction>Run</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )
    case "AlertCenter":
      return (
        <PanelPreviewFrame>
          <AlertCenter />
        </PanelPreviewFrame>
      )
    case "alert":
      return (
        <Alert className="w-full px-2 py-1.5 text-[10px]">
          <AlertTitle className="text-[10px] leading-4">Alert</AlertTitle>
          <AlertDescription className="text-[9px] leading-3">Status message</AlertDescription>
        </Alert>
      )
    case "animated-modal":
      return (
        <Modal>
          <ModalTrigger className="border bg-background text-xs text-foreground">Open modal</ModalTrigger>
          <ModalBody>
            <ModalContent><div className="text-sm font-medium">Animated modal</div></ModalContent>
            <ModalFooter><Button size="sm">Done</Button></ModalFooter>
          </ModalBody>
        </Modal>
      )
    case "aspect-ratio":
      return (
        <AspectRatio ratio={16 / 9} className="overflow-hidden rounded-md bg-primary/20">
          <div className="flex size-full items-center justify-center text-[10px] text-muted-foreground">16:9</div>
        </AspectRatio>
      )
    case "avatar":
      return (
        <AvatarGroup className="justify-center">
          <Avatar><AvatarFallback>ER</AvatarFallback></Avatar>
          <Avatar><AvatarFallback>AI</AvatarFallback></Avatar>
          <AvatarGroupCount>+3</AvatarGroupCount>
        </AvatarGroup>
      )
    case "badge":
      return <Badge variant="outline">Badge</Badge>
    case "breadcrumb":
      return (
        <Breadcrumb>
          <BreadcrumbList className="justify-center text-xs">
            <BreadcrumbItem><BreadcrumbLink>node</BreadcrumbLink></BreadcrumbItem>
            <BreadcrumbSeparator />
            <BreadcrumbItem><BreadcrumbPage>components</BreadcrumbPage></BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      )
    case "button":
      return <Button size="sm">Button</Button>
    case "moving-border":
      return (
        <MovingBorderButton containerClassName="h-10 w-28 text-sm" borderRadius="0.5rem">
          Border
        </MovingBorderButton>
      )
    case "button-group":
      return (
        <ButtonGroup className="h-8 px-2">
          <ButtonGroupText>A</ButtonGroupText>
          <ButtonGroupSeparator />
          <ButtonGroupText>B</ButtonGroupText>
        </ButtonGroup>
      )
    case "CapabilityList":
      return (
        <PanelPreviewFrame>
          <CapabilityList />
        </PanelPreviewFrame>
      )
    case "capability-gate-prompt":
      return (
        <PanelPreviewFrame>
          <CapabilityGatePrompt
            appName="Storage"
            blockedCapabilities={["storage:read", "storage:write"]}
            onDismiss={() => undefined}
            onGrant={() => undefined}
          />
        </PanelPreviewFrame>
      )
    case "calendar":
      return <Calendar mode="single" selected={new Date(2026, 4, 9)} className="scale-[0.52] rounded-md border bg-background shadow-sm" />
    case "card":
      return (
        <Card className="w-full gap-2 p-3">
          <CardHeader className="p-0"><CardTitle className="text-xs">Card</CardTitle><CardDescription className="text-[10px]">Surface</CardDescription></CardHeader>
          <CardContent className="p-0 text-[10px] text-muted-foreground">Content</CardContent>
        </Card>
      )
    case "carousel":
      return (
        <Carousel className="w-full max-w-36">
          <CarouselContent>
            {[1, 2, 3].map((value) => (
              <CarouselItem key={value}>
                <div className="flex h-16 items-center justify-center rounded-md border bg-muted text-sm">{value}</div>
              </CarouselItem>
            ))}
          </CarouselContent>
          <CarouselPrevious className="-left-3 size-6" />
          <CarouselNext className="-right-3 size-6" />
        </Carousel>
      )
    case "checkbox":
      return <Checkbox defaultChecked />
    case "code-block":
      return (
        <div className="max-h-28 w-full overflow-hidden rounded-md text-left">
          <CodeBlock language="tsx" filename="node.tsx" code={"export function Node() {\n  return <Button />\n}"} highlightLines={[2]} />
        </div>
      )
    case "collapsible":
      return (
        <Collapsible defaultOpen className="w-full text-left text-xs">
          <CollapsibleTrigger className="rounded-md border px-2 py-1">Collapsible</CollapsibleTrigger>
          <CollapsibleContent className="mt-2 text-[10px] text-muted-foreground">Open content</CollapsibleContent>
        </Collapsible>
      )
    case "codelyzer-network":
      return (
        <PanelPreviewFrame>
          <CodelyzerNetworkPanel />
          <div className="mt-3 grid grid-cols-1 gap-2">
            <NetworkConnectionsWidget />
            <CodelyzerCodeWidget />
          </div>
        </PanelPreviewFrame>
      )
    case "combobox":
      return (
        <Combobox items={["Storage", "Auth", "Mail"]}>
          <ComboboxInput placeholder="Combobox" />
          <ComboboxContent>
            <ComboboxList>
              <ComboboxItem value="Storage">Storage</ComboboxItem>
              <ComboboxItem value="Auth">Auth</ComboboxItem>
              <ComboboxItem value="Mail">Mail</ComboboxItem>
            </ComboboxList>
          </ComboboxContent>
        </Combobox>
      )
    case "command":
      return (
        <Command className="h-28 border">
          <CommandInput placeholder="Command" />
          <CommandList>
            <CommandGroup heading="Node">
              <CommandItem>Open dashboard<CommandShortcut>⌘D</CommandShortcut></CommandItem>
              <CommandItem>Sync storage</CommandItem>
            </CommandGroup>
          </CommandList>
        </Command>
      )
    case "context-menu":
      return (
        <ContextMenu>
          <ContextMenuTrigger className="rounded-md border px-3 py-2 text-xs">Right click</ContextMenuTrigger>
          <ContextMenuContent>
            <ContextMenuItem>Inspect</ContextMenuItem>
            <ContextMenuItem>Duplicate</ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>
      )
    case "DependencyGraph":
      return (
        <PanelPreviewFrame>
          <DependencyGraph />
        </PanelPreviewFrame>
      )
    case "dialog":
      return (
        <Dialog>
          <DialogTrigger asChild><Button size="sm" variant="outline">Dialog</Button></DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Dialog</DialogTitle>
              <DialogDescription>Portal content preview.</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>
      )
    case "direction":
      return (
        <DirectionProvider dir="rtl">
          <div className="w-full rounded-md border p-2 text-right text-xs">RTL direction</div>
        </DirectionProvider>
      )
    case "drawer":
      return (
        <Drawer>
          <DrawerTrigger asChild><Button size="sm" variant="outline">Drawer</Button></DrawerTrigger>
          <DrawerContent>
            <DrawerHeader><DrawerTitle>Drawer</DrawerTitle><DrawerDescription>Bottom panel</DrawerDescription></DrawerHeader>
          </DrawerContent>
        </Drawer>
      )
    case "dropdown-menu":
      return (
        <DropdownMenu>
          <DropdownMenuTrigger asChild><Button size="sm" variant="outline">Menu</Button></DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel>Actions</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem>Open</DropdownMenuItem>
            <DropdownMenuItem>Share</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )
    case "finance-overviews":
      return (
        <PanelPreviewFrame>
          <FinancesOverviewWidget />
        </PanelPreviewFrame>
      )
    case "empty":
      return (
        <Empty className="gap-2 p-0">
          <EmptyHeader>
            <EmptyMedia><Database className="size-4" /></EmptyMedia>
            <EmptyTitle className="text-xs">Empty</EmptyTitle>
            <EmptyDescription className="text-[10px]">No records</EmptyDescription>
          </EmptyHeader>
          <EmptyContent />
        </Empty>
      )
    case "field":
      return (
        <FieldGroup className="gap-2 text-left">
          <Field>
            <FieldLabel className="text-xs">Field</FieldLabel>
            <FieldContent><FieldTitle className="text-xs">Title</FieldTitle><FieldDescription className="text-[10px]">Description</FieldDescription></FieldContent>
          </Field>
        </FieldGroup>
      )
    case "form":
      return <FormFixture />
    case "glowing-effect":
      return (
        <div className="relative h-20 w-full rounded-md border p-3">
          <GlowingEffect disabled={false} glow proximity={90} spread={32} />
          <div className="relative z-10 text-xs font-medium">Glowing effect</div>
        </div>
      )
    case "glowing-stars":
      return (
        <GlowingStarsBackgroundCard className="h-32 max-h-32 rounded-md p-2">
          <GlowingStarsTitle className="text-sm">Stars</GlowingStarsTitle>
          <GlowingStarsDescription className="text-[10px]">Animated card</GlowingStarsDescription>
        </GlowingStarsBackgroundCard>
      )
    case "grid-view":
      return (
        <div className="grid w-full grid-cols-3 gap-1">
          {Array.from({ length: 6 }).map((_, index) => (
            <div key={index} className="aspect-square rounded-sm bg-primary/20" />
          ))}
        </div>
      )
    case "hover-card":
      return (
        <HoverCard>
          <HoverCardTrigger className="rounded-md border px-3 py-2 text-xs">Hover</HoverCardTrigger>
          <HoverCardContent className="text-xs">Hover card content</HoverCardContent>
        </HoverCard>
      )
    case "input":
      return <Input className="h-8" placeholder="Input" />
    case "input-group":
      return (
        <InputGroup>
          <InputGroupAddon><InputGroupText>@</InputGroupText></InputGroupAddon>
          <InputGroupInput placeholder="group" />
        </InputGroup>
      )
    case "input-otp":
      return (
        <InputOTP maxLength={4} value="12">
          <InputOTPGroup>
            <InputOTPSlot index={0} />
            <InputOTPSlot index={1} />
            <InputOTPSlot index={2} />
            <InputOTPSlot index={3} />
          </InputOTPGroup>
        </InputOTP>
      )
    case "item":
      return (
        <ItemGroup className="w-full">
          <Item className="p-2">
            <ItemMedia><Database className="size-4" /></ItemMedia>
            <ItemContent><ItemTitle className="text-xs">Item</ItemTitle><ItemDescription className="text-[10px]">Description</ItemDescription></ItemContent>
          </Item>
        </ItemGroup>
      )
    case "kbd":
      return <KbdGroup><Kbd>⌘</Kbd><Kbd>K</Kbd></KbdGroup>
    case "label":
      return <Label>Label</Label>
    case "layout-grid":
      return (
        <div className="h-28 w-full overflow-hidden rounded-md">
          <LayoutGrid cards={layoutGridCards} />
        </div>
      )
    case "node-dashboard":
      return (
        <PanelPreviewFrame>
          <div className="grid grid-cols-3 gap-1">
            <div className="h-12 rounded bg-primary/20" />
            <div className="h-12 rounded bg-emerald-500/20" />
            <div className="h-12 rounded bg-amber-500/20" />
          </div>
          <div className="mt-2 text-xs text-muted-foreground">Current dashboard surface</div>
        </PanelPreviewFrame>
      )
    case "menubar":
      return (
        <Menubar>
          <MenubarMenu>
            <MenubarTrigger>File</MenubarTrigger>
            <MenubarContent>
              <MenubarItem>New</MenubarItem>
              <MenubarItem>Open</MenubarItem>
            </MenubarContent>
          </MenubarMenu>
          <MenubarMenu>
            <MenubarTrigger>View</MenubarTrigger>
            <MenubarContent><MenubarItem>Grid</MenubarItem></MenubarContent>
          </MenubarMenu>
        </Menubar>
      )
    case "multi-step-loader":
      return (
        <div className="flex flex-col items-center gap-2">
          <MultiStepLoader loadingStates={[{ text: "Connect" }, { text: "Sync" }]} loading={false} />
          <Badge variant="outline" className="rounded-md">loader</Badge>
        </div>
      )
    case "native-select":
      return (
        <NativeSelect className="h-8">
          <NativeSelectOption>Native</NativeSelectOption>
        </NativeSelect>
      )
    case "navigation-menu":
      return (
        <NavigationMenu viewport={false}>
          <NavigationMenuList>
            <NavigationMenuItem>
              <NavigationMenuTrigger>Node</NavigationMenuTrigger>
              <NavigationMenuContent>
                <NavigationMenuLink className="w-28">Dashboard</NavigationMenuLink>
              </NavigationMenuContent>
            </NavigationMenuItem>
          </NavigationMenuList>
        </NavigationMenu>
      )
    case "pagination":
      return (
        <Pagination>
          <PaginationContent>
            <PaginationItem><PaginationPrevious href="#" /></PaginationItem>
            <PaginationItem><PaginationLink href="#" isActive>1</PaginationLink></PaginationItem>
            <PaginationItem><PaginationEllipsis /></PaginationItem>
            <PaginationItem><PaginationNext href="#" /></PaginationItem>
          </PaginationContent>
        </Pagination>
      )
    case "PipelineProgress":
      return (
        <PanelPreviewFrame>
          <PipelineProgress />
        </PanelPreviewFrame>
      )
    case "popover":
      return (
        <Popover>
          <PopoverTrigger asChild><Button size="sm" variant="outline">Popover</Button></PopoverTrigger>
          <PopoverContent>
            <PopoverHeader>
              <PopoverTitle>Resource</PopoverTitle>
              <PopoverDescription>CPU, memory, storage.</PopoverDescription>
            </PopoverHeader>
          </PopoverContent>
        </Popover>
      )
    case "react-shader-toy":
      return (
        <ReactShaderToy
          className="h-24 w-full rounded-md"
          fs={"void mainImage(out vec4 fragColor, in vec2 fragCoord){ vec2 uv=fragCoord/iResolution.xy; fragColor=vec4(uv.x, uv.y, 0.8, 1.0); }"}
        />
      )
    case "progress":
      return <Progress value={64} />
    case "radio-group":
      return (
        <RadioGroup defaultValue="one" className="flex justify-center gap-3">
          <RadioGroupItem value="one" />
          <RadioGroupItem value="two" />
        </RadioGroup>
      )
    case "resizable":
      return (
        <ResizablePanelGroup orientation="horizontal" className="h-12 rounded-md border">
          <ResizablePanel defaultSize={50}><div className="size-full bg-muted/40" /></ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize={50}><div className="size-full bg-primary/15" /></ResizablePanel>
        </ResizablePanelGroup>
      )
    case "scroll-area":
      return (
        <ScrollArea className="h-16 w-full rounded-md border p-2 text-xs">
          <div>Scrollable</div><div>content</div><div>area</div><div>more</div>
        </ScrollArea>
      )
    case "settings-dialog":
      return (
        <Dialog>
          <DialogTrigger asChild><Button size="sm" variant="outline">Settings</Button></DialogTrigger>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>Settings dialog</DialogTitle>
              <DialogDescription>The full component opens by default, so the fixture uses the same dialog primitives without auto-opening.</DialogDescription>
            </DialogHeader>
          </DialogContent>
        </Dialog>
      )
    case "select":
      return (
        <Select defaultValue="storage">
          <SelectTrigger size="sm"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="storage">Storage</SelectItem>
            <SelectItem value="auth">Auth</SelectItem>
          </SelectContent>
        </Select>
      )
    case "separator":
      return <Separator />
    case "sheet":
      return (
        <Sheet>
          <SheetTrigger asChild><Button size="sm" variant="outline">Sheet</Button></SheetTrigger>
          <SheetContent>
            <SheetHeader><SheetTitle>Sheet</SheetTitle><SheetDescription>Side panel</SheetDescription></SheetHeader>
          </SheetContent>
        </Sheet>
      )
    case "sidebar":
      return (
        <div className="h-28 w-full overflow-hidden rounded-md border bg-sidebar">
          <SidebarProvider className="min-h-0">
            <Sidebar collapsible="none" className="h-28 w-full">
              <SidebarContent>
                <SidebarGroup>
                  <SidebarGroupLabel>Node</SidebarGroupLabel>
                  <SidebarGroupContent>
                    <SidebarMenu>
                      <SidebarMenuItem><SidebarMenuButton isActive>Dashboard</SidebarMenuButton></SidebarMenuItem>
                      <SidebarMenuItem><SidebarMenuButton>Storage</SidebarMenuButton></SidebarMenuItem>
                    </SidebarMenu>
                  </SidebarGroupContent>
                </SidebarGroup>
              </SidebarContent>
            </Sidebar>
          </SidebarProvider>
        </div>
      )
    case "TestStatusPanel":
      return (
        <PanelPreviewFrame>
          <TestStatusPanel />
        </PanelPreviewFrame>
      )
    case "skeleton":
      return <div className="w-full space-y-2"><Skeleton className="h-4 w-full" /><Skeleton className="h-4 w-2/3" /></div>
    case "slider":
      return <Slider defaultValue={[40]} max={100} />
    case "spinner":
      return <Spinner className="size-6" />
    case "switch":
      return <Switch defaultChecked />
    case "table":
      return (
        <Table>
          <TableHeader><TableRow><TableHead className="h-6 text-[10px]">Name</TableHead><TableHead className="h-6 text-[10px]">State</TableHead></TableRow></TableHeader>
          <TableBody><TableRow><TableCell className="p-1 text-[10px]">Node</TableCell><TableCell className="p-1 text-[10px]">On</TableCell></TableRow></TableBody>
        </Table>
      )
    case "tabs":
      return (
        <Tabs defaultValue="one" className="w-full">
          <TabsList className="h-8"><TabsTrigger value="one" className="text-xs">One</TabsTrigger><TabsTrigger value="two" className="text-xs">Two</TabsTrigger></TabsList>
          <TabsContent value="one" className="pt-2 text-[10px] text-muted-foreground">Tab content</TabsContent>
        </Tabs>
      )
    case "textarea":
      return <Textarea className="min-h-14 text-xs" defaultValue="Textarea" />
    case "toggle":
      return <Toggle defaultPressed>Toggle</Toggle>
    case "toggle-group":
      return (
        <ToggleGroup type="single" defaultValue="a">
          <ToggleGroupItem value="a">A</ToggleGroupItem>
          <ToggleGroupItem value="b">B</ToggleGroupItem>
        </ToggleGroup>
      )
    case "TokenUsagePanel":
      return (
        <PanelPreviewFrame>
          <TokenUsagePanel />
        </PanelPreviewFrame>
      )
    case "toast":
      return (
        <div className="w-full rounded-md border bg-background p-3 text-left shadow">
          <div className="text-xs font-medium">Toast</div>
          <div className="text-[10px] text-muted-foreground">Notification body</div>
        </div>
      )
    case "tooltip":
      return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild><Button size="sm" variant="outline">Tooltip</Button></TooltipTrigger>
            <TooltipContent>Tooltip content</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )
    case "use-mobile":
      return <HookPreview name="useIsMobile" />
    case "use-toast":
      return <HookPreview name="useToast" />
    case "wasm-installer":
      return (
        <PanelPreviewFrame>
          <WasmInstaller onCancel={() => undefined} onInstall={() => undefined} />
        </PanelPreviewFrame>
      )
    case "world-map":
      return (
        <div className="w-full overflow-hidden rounded-md">
          <WorldMap
            lineColor="#38bdf8"
            dots={[{ start: { lat: 13.7563, lng: 100.5018 }, end: { lat: 37.7749, lng: -122.4194 } }]}
          />
        </div>
      )
    case "workspace-prompt-input":
      return (
        <div className="relative h-24 w-full overflow-hidden rounded-md border">
          <WorkspacePromptInput
            className="static max-w-full translate-x-0 p-2 opacity-100"
            onSubmit={() => undefined}
            placeholder="Compose..."
            suggestions={[{ id: "query", label: "Query", action: () => undefined }]}
          />
        </div>
      )
    case "workspace-status-panel":
      return (
        <PanelPreviewFrame>
          <WorkspaceStatusPanel surface="embedded" />
        </PanelPreviewFrame>
      )
    case "XrayCommandSurface":
      return (
        <PanelPreviewFrame>
          <XrayCommandSurface surface="inline" />
        </PanelPreviewFrame>
      )
    case "XrayDashboard":
      return (
        <PanelPreviewFrame>
          <div className="text-xs font-medium">Xray dashboard</div>
          <div className="mt-2 h-20 rounded-md border bg-[radial-gradient(circle_at_50%_50%,rgba(56,189,248,0.28),transparent_32%),linear-gradient(135deg,rgba(255,255,255,0.08),transparent)]" />
        </PanelPreviewFrame>
      )
    case "XrayInspector":
      return (
        <div className="h-36 overflow-hidden rounded-md border">
          <XrayInspector />
        </div>
      )
    case "XrayOverlayWidgets":
      return (
        <div className="relative h-36 w-full overflow-hidden rounded-md border bg-zinc-950">
          <XrayLegendOverlay />
          <XrayMouseHelpOverlay />
        </div>
      )
    case "XrayViewport":
      return (
        <div className="h-36 w-full overflow-hidden rounded-md border bg-zinc-950">
          <XrayViewport />
        </div>
      )
    case "XrayWorkspace":
      return (
        <div className="h-36 w-full overflow-hidden rounded-md border">
          <XrayWorkspace mode="bg" />
        </div>
      )
    case "codeanalyzer-ws":
    case "color-policy":
    case "force-layout":
    case "globe-layout":
    case "graph-commands":
    case "graph-store":
    case "layer-layout":
    case "mock-graph":
    case "morph-layout":
    case "runtime-store":
    case "shaders":
    case "types":
    case "webgl-renderer":
    case "xray-wire-wasm":
      return <HookPreview name="xray module" />
    case "index":
    case "theme-provider":
      return <HookPreview name="module exports" />
    default:
      return (
        <span className="flex flex-col items-center gap-2">
          <span className="flex size-12 items-center justify-center rounded-md bg-white/10 text-white shadow-md transition group-hover:bg-primary/20">
            <Icon className="size-5" />
          </span>
          <span className="max-w-full truncate font-mono text-[10px] text-muted-foreground/75">
            fixture needed
          </span>
        </span>
      )
  }
}

export function NodeDashboard() {
  const fs = useStore(fileSystemStore)
  const codebase = useStore(codebaseStore)
  const [workspaceMessage, setWorkspaceMessage] = React.useState<string | null>(null)
  const [fileTreeVisible, setFileTreeVisible] = React.useState(false)
  const [fileTreeWidth, setFileTreeWidth] = React.useState(248)
  const [selectedTreePaths, setSelectedTreePaths] = React.useState<string[]>([])
  const [lastSelectedTreePath, setLastSelectedTreePath] = React.useState<string | null>(null)
  const [fileSortMode, setFileSortMode] = React.useState<FileSortMode>("name")
  const [fileShowMode, setFileShowMode] = React.useState<FileShowMode>("all")
  const [fileGroupMode, setFileGroupMode] = React.useState<FileGroupMode>("none")
  const [fileViewMode, setFileViewMode] = React.useState<FileViewMode>("chart")
  const [codeMetadata, setCodeMetadata] = React.useState<Record<string, FileCodeMeta>>({})
  const [analysisProgress, setAnalysisProgress] = React.useState<AnalyzerProgress>({
    active: false,
    phase: "idle",
    completed: 0,
    total: 0,
    label: "",
  })
  const analysisWorkerRef = React.useRef<Worker | null>(null)
  const folderInputRef = React.useRef<HTMLInputElement>(null)
  const workspaceName = fs.repoName || fs.rootPath || codebase.rootPath
  const workspaceGridEntries = React.useMemo(
    () => orderFileEntries(fs.entries, fileSortMode, fileShowMode, fileGroupMode),
    [fs.entries, fileSortMode, fileShowMode, fileGroupMode]
  )

  React.useEffect(() => {
    updateDependencyGraph(buildWorkspaceDependencyGraph(fs.entries, codeMetadata))
  }, [fs.entries, codeMetadata])

  React.useEffect(() => {
    const next: Record<string, FileCodeMeta> = {}
    for (const file of fs.openFiles) {
      if (file.binary) continue
      next[file.path] = {
        imports: extractImports(file.content),
        importedBy: [],
        linesOfCode: file.content.split(/\r\n|\r|\n/).filter((line) => line.trim().length > 0).length,
        functions: extractFunctions(file.content, file.path),
      }
    }

    const paths = Object.keys(next)
    for (const sourcePath of paths) {
      for (const imported of next[sourcePath].imports) {
        const target = paths.find((candidate) => importMatchesPath(imported, candidate))
        if (target) next[target].importedBy.push(sourcePath)
      }
    }

    setCodeMetadata(next)
  }, [fs.openFiles])

  const startWorkspaceAnalysis = React.useCallback((rootHandle: FileSystemDirectoryHandle | null) => {
    if (!rootHandle || typeof Worker === "undefined") return

    analysisWorkerRef.current?.terminate()
    const worker = new Worker("/workers/workspace-analyzer-worker.js")
    analysisWorkerRef.current = worker
    setAnalysisProgress({
      active: true,
      phase: "compiling",
      completed: 0,
      total: 1,
      label: "Compiling codelyzer WASM",
    })

    worker.onmessage = (event: MessageEvent) => {
      const message = event.data
      if (message?.type === "progress") {
        setAnalysisProgress({
          active: true,
          phase: message.phase,
          completed: message.completed,
          total: message.total,
          label: message.label,
        })
        return
      }

      if (message?.type === "done") {
        setCodeMetadata(message.metadata ?? {})
        setAnalysisProgress({
          active: true,
          phase: "done",
          completed: message.total ?? 1,
          total: message.total ?? 1,
          label: `Analyzed ${message.analyzedFiles ?? 0} code files from ${message.total ?? 0} entries`,
        })
        window.setTimeout(() => {
          setAnalysisProgress((current) => current.phase === "done" ? { ...current, active: false } : current)
        }, 2500)
        worker.terminate()
        if (analysisWorkerRef.current === worker) analysisWorkerRef.current = null
        return
      }

      if (message?.type === "error") {
        setAnalysisProgress({
          active: true,
          phase: "error",
          completed: 1,
          total: 1,
          label: message.error ?? "Workspace analysis failed",
        })
        worker.terminate()
        if (analysisWorkerRef.current === worker) analysisWorkerRef.current = null
      }
    }

    worker.onerror = (event) => {
      setAnalysisProgress({
        active: true,
        phase: "error",
        completed: 1,
        total: 1,
        label: event.message,
      })
      worker.terminate()
      if (analysisWorkerRef.current === worker) analysisWorkerRef.current = null
    }

    worker.postMessage({ type: "analyze", rootHandle })
  }, [])

  React.useEffect(() => {
    return () => analysisWorkerRef.current?.terminate()
  }, [])

  React.useEffect(() => {
    folderInputRef.current?.setAttribute("webkitdirectory", "")
    folderInputRef.current?.setAttribute("directory", "")
  }, [])

  React.useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "b") return
      event.preventDefault()
      setFileTreeVisible((current) => !current)
    }

    window.addEventListener("keydown", onKeyDown, { capture: true })
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true })
  }, [])

  const startFileTreeResize = React.useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    const startX = event.clientX
    const startWidth = fileTreeWidth
    const previousCursor = document.body.style.cursor
    const previousUserSelect = document.body.style.userSelect
    document.body.style.cursor = "col-resize"
    document.body.style.userSelect = "none"

    const onPointerMove = (moveEvent: PointerEvent) => {
      const nextWidth = Math.min(380, Math.max(208, startWidth + moveEvent.clientX - startX))
      setFileTreeWidth(nextWidth)
    }

    const onPointerUp = () => {
      document.body.style.cursor = previousCursor
      document.body.style.userSelect = previousUserSelect
      window.removeEventListener("pointermove", onPointerMove)
      window.removeEventListener("pointerup", onPointerUp)
    }

    window.addEventListener("pointermove", onPointerMove)
    window.addEventListener("pointerup", onPointerUp, { once: true })
  }, [fileTreeWidth])

  const handleOpenWorkspace = React.useCallback(async () => {
    if (!isFileSystemAccessSupported()) {
      setWorkspaceMessage("Brave hid showDirectoryPicker; using folder upload fallback...")
      folderInputRef.current?.click()
      return
    }

    setWorkspaceMessage("Opening folder picker...")
    const opened = await openDirectory()

    if (!opened) {
      const nextError = fileSystemStore.get().error || fileSystemStore.get().repoIndexError || "Folder was not opened"
      setWorkspaceMessage(nextError)
      return
    }

    setWorkspaceMessage("Indexing workspace context...")
    await scanCodebase()

    const nextFs = fileSystemStore.get()
    setFileTreeVisible(true)
    setWorkspaceMessage(`${nextFs.repoName || nextFs.rootPath} is in workspace context`)
    startWorkspaceAnalysis(nextFs.rootHandle)
  }, [startWorkspaceAnalysis])

  const handleFallbackFolder = React.useCallback(async (event: React.ChangeEvent<HTMLInputElement>) => {
    const files = event.target.files
    if (!files?.length) {
      setWorkspaceMessage("No folder selected")
      return
    }

    setWorkspaceMessage("Indexing selected folder...")
    const opened = await openDirectoryFromFileList(files)
    event.target.value = ""

    if (!opened) {
      setWorkspaceMessage("Folder was not opened")
      return
    }

    await scanCodebase()
    const nextFs = fileSystemStore.get()
    setFileTreeVisible(true)
    setWorkspaceMessage(`${nextFs.repoName || nextFs.rootPath} is in workspace context`)
  }, [])

  const dockItems = React.useMemo(
    () => createDockItems(handleOpenWorkspace, workspaceName),
    [handleOpenWorkspace, workspaceName]
  )

  const handleTreeSelect = React.useCallback((entry: FileEntry, event: React.MouseEvent, visibleEntries: FileEntry[]) => {
    const additive = event.ctrlKey || event.metaKey

    if (event.shiftKey && lastSelectedTreePath) {
      const start = visibleEntries.findIndex((candidate) => candidate.path === lastSelectedTreePath)
      const end = visibleEntries.findIndex((candidate) => candidate.path === entry.path)
      if (start >= 0 && end >= 0) {
        const [from, to] = start <= end ? [start, end] : [end, start]
        const range = visibleEntries.slice(from, to + 1).map((candidate) => candidate.path)
        setSelectedTreePaths(additive ? Array.from(new Set([...selectedTreePaths, ...range])) : range)
        return
      }
    }

    setLastSelectedTreePath(entry.path)
    setSelectedTreePaths((current) => {
      if (!additive) return [entry.path]
      return current.includes(entry.path)
        ? current.filter((path) => path !== entry.path)
        : [...current, entry.path]
    })
  }, [lastSelectedTreePath, selectedTreePaths])

  const workspaceTree = (
    <WorkspaceFileTree
      entries={fs.entries}
      expandedDirs={fs.expandedDirs}
      rootName={workspaceName ?? "Workspace"}
      gitInfo={fs.gitInfo}
      isGitRepo={fs.isGitRepo}
      visible={Boolean(workspaceName) && fileTreeVisible}
      onToggleVisible={() => setFileTreeVisible((current) => !current)}
      selectedPaths={selectedTreePaths}
      onSelect={handleTreeSelect}
      sortMode={fileSortMode}
      showMode={fileShowMode}
      groupMode={fileGroupMode}
      viewMode={fileViewMode}
      onSortModeChange={setFileSortMode}
      onShowModeChange={setFileShowMode}
      onGroupModeChange={setFileGroupMode}
      onViewModeChange={setFileViewMode}
    />
  )

  const workspaceMain = fileViewMode === "chart" ? (
    <WorkspaceCompositionView entries={workspaceGridEntries} metadata={codeMetadata} isGitRepo={fs.isGitRepo} gitInfo={fs.gitInfo} />
  ) : fileViewMode === "table" ? (
    <WorkspaceTableView
      entries={workspaceGridEntries}
      selectedPaths={selectedTreePaths}
      onSelect={handleTreeSelect}
      metadata={codeMetadata}
    />
  ) : (
    <GridView
      className="h-full px-6 pb-32 pt-6 sm:px-8"
      itemMinSize={132}
      minItemSize={82}
      maxItemSize={1200}
    >
      {workspaceGridEntries.map((entry) => (
        <div
          key={entry.path}
          title={entry.path}
          role="button"
          tabIndex={0}
          className={`group flex aspect-square min-h-0 flex-col rounded-md border p-3 text-center shadow-lg backdrop-blur-sm transition hover:border-primary/35 hover:bg-primary/10 ${
            selectedTreePaths.includes(entry.path)
              ? "border-primary/45 bg-primary/15"
              : "border-white/10 bg-white/[0.045]"
          }`}
          onClick={(event) => {
            handleTreeSelect(entry, event, workspaceGridEntries)
            if (entry.kind === "directory") void toggleDirectory(entry.path)
            else void openFile(entry.path)
          }}
        >
          <FileGridPreview entry={entry} entries={fs.entries} />
        </div>
      ))}
    </GridView>
  )

  const componentGrid = (
    <GridView
      className="absolute inset-x-0 bottom-28 top-0 z-10 px-6 pb-8 pt-8 sm:px-10"
      itemMinSize={132}
      minItemSize={82}
      maxItemSize={1200}
    >
      {sideEffectSafeEntries.map((entry) => {
        const Icon = groupIcon(entry.group)
        const title = componentTitle(entry)

        return (
          <div
            key={entry.path}
            title={entry.path}
            role="button"
            tabIndex={0}
            className="group flex aspect-square min-h-0 flex-col rounded-md border border-white/10 bg-white/[0.045] p-3 text-center shadow-lg backdrop-blur-sm transition hover:border-primary/35 hover:bg-primary/10"
          >
            <span className="flex min-h-0 flex-1 items-center justify-center overflow-hidden rounded-md border border-white/10 bg-black/15 p-2">
              <ComponentPreview entry={entry} title={title} icon={Icon} />
            </span>
            <span className="mt-3 min-w-0">
              <span className="block truncate text-sm font-medium">{title}</span>
              <span className="mt-1 block truncate text-xs text-muted-foreground">{entry.group}</span>
              <span className="mt-1 block truncate font-mono text-[10px] text-muted-foreground/80">
                {entry.exports.length} exports
              </span>
            </span>
          </div>
        )
      })}
    </GridView>
  )

  return (
    <main className="relative h-svh w-screen overflow-hidden bg-[oklch(0.075_0.018_255)] text-foreground">
      <input
        ref={folderInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={handleFallbackFolder}
      />
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_36%,rgba(62,128,255,0.16),transparent_34%),linear-gradient(180deg,rgba(255,255,255,0.025),transparent_30%,rgba(0,0,0,0.22))]" />
      <WorkspaceProgress progress={analysisProgress} />
      {workspaceName ? (
        <div
          className="relative z-10 grid h-full min-h-0 bg-background text-foreground"
          style={fileTreeVisible ? { gridTemplateColumns: `${fileTreeWidth}px 4px minmax(0,1fr)` } : { gridTemplateColumns: "minmax(0,1fr)" }}
        >
          {fileTreeVisible && workspaceTree}
          {fileTreeVisible && (
            <div
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize file tree"
              className="z-20 cursor-col-resize border-r border-white/10 bg-[oklch(0.085_0.014_255)] transition-colors hover:bg-primary/30"
              onPointerDown={startFileTreeResize}
            />
          )}
          {!fileTreeVisible && (
            <div className="absolute left-4 top-4 z-20">
              <Button type="button" size="sm" variant="outline" className="h-8 border-white/10 bg-background/85 text-xs shadow-xl backdrop-blur-xl" onClick={() => setFileTreeVisible(true)}>
                Show files
              </Button>
            </div>
          )}
          <section className="min-h-0 min-w-0 overflow-hidden bg-[oklch(0.09_0.014_255)]">
            {workspaceMain}
          </section>
        </div>
      ) : (
        <>
          {workspaceMessage && (
            <div className="pointer-events-none absolute left-6 top-5 z-40 max-w-sm rounded-md border border-white/10 bg-background/75 px-3 py-2 text-xs shadow-xl backdrop-blur-xl sm:left-10">
              <div className="flex items-center gap-2 font-medium">
                <FolderOpen className="size-3.5 text-primary" />
                <span className="truncate">Workspace</span>
              </div>
              <div className="mt-1 truncate text-muted-foreground">{workspaceMessage}</div>
            </div>
          )}
          {componentGrid}
        </>
      )}
      <FloatingDock
        items={dockItems}
        context={dockContext}
        desktopClassName="fixed bottom-5 left-1/2 z-50 -translate-x-1/2"
        mobileClassName="fixed bottom-5 left-1/2 z-50 -translate-x-1/2"
      />
    </main>
  )
}
