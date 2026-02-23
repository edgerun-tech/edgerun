/**
 * Icon Registry - Centralized Icon Management
 * 
 * Standardizes iconography across the platform using Tabler Icons (outline style)
 * for a consistent, modern aesthetic.
 * 
 * Usage:
 *   import { icons } from '@/lib/icons';
 *   <icons.ai size={20} />
 */

import type { Component } from 'solid-js';

// Tabler Icons
import {
  // System & Navigation
  TbOutlineHome,
  TbOutlineGridDots,
  TbOutlineApps,
  TbOutlineSettings,
  TbOutlineSearch,
  TbOutlineMenu,
  TbOutlineX,
  TbOutlineCheck,
  TbOutlinePlus,
  TbOutlineMinus,
  TbOutlineDots,
  TbOutlineDotsVertical,
  
  // Files & Content
  TbOutlineFolder,
  TbOutlineFolderPlus,
  TbOutlineFile,
  TbOutlineFileText,
  TbOutlineClipboard,
  TbOutlineCopy,
  TbOutlineDownload,
  TbOutlineUpload,
  TbOutlineTrash,
  
  // Technology & Code
  TbOutlineTerminal2,
  TbOutlineCode,
  TbOutlineBrandGithub,
  TbOutlineGitBranch,
  TbOutlineGitPullRequest,
  TbOutlineDatabase,
  TbOutlineServer,
  TbOutlineCloud,
  TbOutlineCpu,
  
  // Communication
  TbOutlineMail,
  TbOutlineCalendar,
  TbOutlinePhone,
  TbOutlineMessage,
  TbOutlineUser,
  TbOutlineUsers,
  
  // Cloud & Infrastructure
  TbOutlineCloudBolt,
  TbOutlineWorld,
  TbOutlineActivity,
  TbOutlineChartBar,
  
  // AI & Intelligence
  TbOutlineBrain,
  TbOutlineSparkles,
  TbOutlineRobot,
  
  // Status & Actions
  TbOutlineRefresh,
  TbOutlineClock,
  TbOutlineInfoCircle,
  TbOutlineAlertCircle,
  TbOutlineCircleCheck,
  TbOutlineCircleX,
  TbOutlineLoader,
  
  // Navigation & Layout
  TbOutlineChevronLeft,
  TbOutlineChevronRight,
  TbOutlineChevronUp,
  TbOutlineChevronDown,
  TbOutlineArrowLeft,
  TbOutlineArrowRight,
  TbOutlineArrowUp,
  TbOutlineArrowDown,
  TbOutlineExternalLink,
  TbOutlineLink,
  
  // View & Display
  TbOutlineEye,
  TbOutlineEyeOff,
  TbOutlineMaximize,
  TbOutlineMinimize,
  TbOutlineZoomIn,
  TbOutlineZoomOut,
  TbOutlineLayoutGrid,
  TbOutlineList,
  TbOutlineDetails,
  
  // Media
  TbOutlinePhoto,
  TbOutlineMusic,
  TbOutlineMovie,
  TbOutlineVideo,
  TbOutlineMicrophone,
  TbOutlineMicrophoneOff,
  TbOutlineCamera,
  
  // Security
  TbOutlineLock,
  TbOutlineLockOpen,
  TbOutlineShield,
  TbOutlineKey,
  
  // Time
  TbOutlineSun,
  TbOutlineMoon,
  TbOutlineClockHour12,
  
  // Commerce & Business
  TbOutlineShoppingCart,
  TbOutlineCreditCard,
  TbOutlineReceipt,
  
  // Miscellaneous
  TbOutlineStar,
  TbOutlineHeart,
  TbOutlineBookmark,
  TbOutlineTag,
  TbOutlineFlag,
} from 'solid-icons/tb';

// Icon registry mapping string identifiers to components
export const icons: Record<string, Component<any>> = {
  // System & Navigation
  home: TbOutlineHome,
  grid: TbOutlineGridDots,
  apps: TbOutlineApps,
  settings: TbOutlineSettings,
  search: TbOutlineSearch,
  menu: TbOutlineMenu,
  close: TbOutlineX,
  check: TbOutlineCheck,
  plus: TbOutlinePlus,
  minus: TbOutlineMinus,
  more: TbOutlineDots,
  moreVertical: TbOutlineDotsVertical,
  
  // Files & Content
  folder: TbOutlineFolder,
  folderPlus: TbOutlineFolderPlus,
  file: TbOutlineFile,
  fileText: TbOutlineFileText,
  clipboard: TbOutlineClipboard,
  copy: TbOutlineCopy,
  download: TbOutlineDownload,
  upload: TbOutlineUpload,
  trash: TbOutlineTrash,
  
  // Technology & Code
  terminal: TbOutlineTerminal2,
  code: TbOutlineCode,
  github: TbOutlineBrandGithub,
  gitBranch: TbOutlineGitBranch,
  gitPullRequest: TbOutlineGitPullRequest,
  database: TbOutlineDatabase,
  server: TbOutlineServer,
  cloud: TbOutlineCloud,
  cpu: TbOutlineCpu,
  
  // Communication
  mail: TbOutlineMail,
  calendar: TbOutlineCalendar,
  phone: TbOutlinePhone,
  message: TbOutlineMessage,
  user: TbOutlineUser,
  users: TbOutlineUsers,
  
  // Cloud & Infrastructure
  cloudBolt: TbOutlineCloudBolt,
  world: TbOutlineWorld,
  activity: TbOutlineActivity,
  chartBar: TbOutlineChartBar,
  
  // AI & Intelligence
  brain: TbOutlineBrain,
  sparkles: TbOutlineSparkles,
  robot: TbOutlineRobot,
  ai: TbOutlineSparkles,  // Alias for sparkles
  
  // Status & Actions
  refresh: TbOutlineRefresh,
  clock: TbOutlineClock,
  info: TbOutlineInfoCircle,
  alert: TbOutlineAlertCircle,
  checkCircle: TbOutlineCircleCheck,
  xCircle: TbOutlineCircleX,
  loader: TbOutlineLoader,
  
  // Navigation & Layout
  chevronLeft: TbOutlineChevronLeft,
  chevronRight: TbOutlineChevronRight,
  chevronUp: TbOutlineChevronUp,
  chevronDown: TbOutlineChevronDown,
  arrowLeft: TbOutlineArrowLeft,
  arrowRight: TbOutlineArrowRight,
  arrowUp: TbOutlineArrowUp,
  arrowDown: TbOutlineArrowDown,
  externalLink: TbOutlineExternalLink,
  link: TbOutlineLink,
  
  // View & Display
  eye: TbOutlineEye,
  eyeOff: TbOutlineEyeOff,
  maximize: TbOutlineMaximize,
  minimize: TbOutlineMinimize,
  zoomIn: TbOutlineZoomIn,
  zoomOut: TbOutlineZoomOut,
  layoutGrid: TbOutlineLayoutGrid,
  list: TbOutlineList,
  details: TbOutlineDetails,
  
  // Media
  photo: TbOutlinePhoto,
  music: TbOutlineMusic,
  movie: TbOutlineMovie,
  video: TbOutlineVideo,
  microphone: TbOutlineMicrophone,
  microphoneOff: TbOutlineMicrophoneOff,
  camera: TbOutlineCamera,
  
  // Security
  lock: TbOutlineLock,
  lockOpen: TbOutlineLockOpen,
  shield: TbOutlineShield,
  key: TbOutlineKey,
  
  // Time
  sun: TbOutlineSun,
  moon: TbOutlineMoon,
  
  // Commerce & Business
  shoppingCart: TbOutlineShoppingCart,
  creditCard: TbOutlineCreditCard,
  receipt: TbOutlineReceipt,
  
  // Miscellaneous
  star: TbOutlineStar,
  heart: TbOutlineHeart,
  bookmark: TbOutlineBookmark,
  tag: TbOutlineTag,
  flag: TbOutlineFlag,
};

// Export individual icons for direct import
export {
  // System & Navigation
  TbOutlineHome as Home,
  TbOutlineGridDots as Grid,
  TbOutlineApps as Apps,
  TbOutlineSettings as Settings,
  TbOutlineSearch as Search,
  TbOutlineMenu as Menu,
  TbOutlineX as X,
  TbOutlineCheck as Check,
  TbOutlinePlus as Plus,
  TbOutlineMinus as Minus,
  TbOutlineDots as More,
  TbOutlineDotsVertical as MoreVertical,
  
  // Files & Content
  TbOutlineFolder as Folder,
  TbOutlineFolderPlus as FolderPlus,
  TbOutlineFile as File,
  TbOutlineFileText as FileText,
  TbOutlineClipboard as Clipboard,
  TbOutlineCopy as Copy,
  TbOutlineDownload as Download,
  TbOutlineUpload as Upload,
  TbOutlineTrash as Trash,
  
  // Technology & Code
  TbOutlineTerminal2 as Terminal,
  TbOutlineCode as Code,
  TbOutlineBrandGithub as Github,
  TbOutlineGitBranch as GitBranch,
  TbOutlineGitPullRequest as GitPullRequest,
  TbOutlineDatabase as Database,
  TbOutlineServer as Server,
  TbOutlineCloud as Cloud,
  TbOutlineCpu as Cpu,
  
  // Communication
  TbOutlineMail as Mail,
  TbOutlineCalendar as Calendar,
  TbOutlinePhone as Phone,
  TbOutlineMessage as Message,
  TbOutlineUser as User,
  TbOutlineUsers as Users,
  
  // Cloud & Infrastructure
  TbOutlineCloudBolt as CloudBolt,
  TbOutlineWorld as World,
  TbOutlineActivity as Activity,
  TbOutlineChartBar as ChartBar,
  
  // AI & Intelligence
  TbOutlineBrain as Brain,
  TbOutlineSparkles as Sparkles,
  TbOutlineRobot as Robot,
  
  // Status & Actions
  TbOutlineRefresh as Refresh,
  TbOutlineClock as Clock,
  TbOutlineInfoCircle as Info,
  TbOutlineAlertCircle as Alert,
  TbOutlineCircleCheck as CheckCircle,
  TbOutlineCircleX as XCircle,
  TbOutlineLoader as Loader,
  
  // Navigation & Layout
  TbOutlineChevronLeft as ChevronLeft,
  TbOutlineChevronRight as ChevronRight,
  TbOutlineChevronUp as ChevronUp,
  TbOutlineChevronDown as ChevronDown,
  TbOutlineArrowLeft as ArrowLeft,
  TbOutlineArrowRight as ArrowRight,
  TbOutlineArrowUp as ArrowUp,
  TbOutlineArrowDown as ArrowDown,
  TbOutlineExternalLink as ExternalLink,
  TbOutlineLink as Link,
  
  // View & Display
  TbOutlineEye as Eye,
  TbOutlineEyeOff as EyeOff,
  TbOutlineMaximize as Maximize,
  TbOutlineMinimize as Minimize,
  TbOutlineZoomIn as ZoomIn,
  TbOutlineZoomOut as ZoomOut,
  TbOutlineLayoutGrid as LayoutGrid,
  TbOutlineList as List,
  TbOutlineDetails as Details,
  
  // Media
  TbOutlinePhoto as Photo,
  TbOutlineMusic as Music,
  TbOutlineMovie as Movie,
  TbOutlineVideo as Video,
  TbOutlineMicrophone as Microphone,
  TbOutlineMicrophoneOff as MicrophoneOff,
  TbOutlineCamera as Camera,
  
  // Security
  TbOutlineLock as Lock,
  TbOutlineLockOpen as LockOpen,
  TbOutlineShield as Shield,
  TbOutlineKey as Key,
  
  // Time
  TbOutlineSun as Sun,
  TbOutlineMoon as Moon,
};

// Icon size presets
export const iconSizes = {
  xs: 12,
  sm: 14,
  md: 16,
  lg: 20,
  xl: 24,
  xxl: 32,
};

// Icon color presets (for Tailwind classes)
export const iconColors = {
  default: 'text-neutral-400',
  primary: 'text-blue-400',
  success: 'text-green-400',
  warning: 'text-yellow-400',
  danger: 'text-red-400',
  purple: 'text-purple-400',
  orange: 'text-orange-400',
};
