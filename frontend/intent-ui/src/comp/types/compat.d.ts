declare module "solid-js" {
  namespace JSX {
    interface HTMLAttributes<T> {
      [key: string]: unknown;
    }
    interface SVGAttributes<T> {
      [key: string]: unknown;
    }
    interface PathSVGAttributes<T> {
      [key: string]: unknown;
    }
    interface StopSVGAttributes<T> {
      [key: string]: unknown;
    }
    interface CircleSVGAttributes<T> {
      [key: string]: unknown;
    }
    interface ForeignObjectSVGAttributes<T> {
      [key: string]: unknown;
    }
    interface InputHTMLAttributes<T> {
      [key: string]: unknown;
    }
    interface TextareaHTMLAttributes<T> {
      [key: string]: unknown;
    }
    interface CSSProperties {
      [key: string]: unknown;
    }
    interface IntrinsicElements {
      [key: string]: Record<string, unknown>;
      "x-placeholder": Record<string, unknown>;
      "el-dialog-backdrop": Record<string, unknown>;
      "el-dropdown": Record<string, unknown>;
      "el-menu": Record<string, unknown>;
      "el-disclosure": Record<string, unknown>;
      "el-option": Record<string, unknown>;
      "el-selectedcontent": Record<string, unknown>;
      "el-select": Record<string, unknown>;
      "el-options": Record<string, unknown>;
    }
  }
}

declare global {
  const ArrowPathIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const Bars3Icon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const BoltIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const BookOpenIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const BookmarkSquareIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const BugAntIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const BuildingOffice2Icon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const CalendarDaysIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ChartPieIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ChatBubbleLeftRightIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ChatBubbleOvalLeftEllipsisIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const CheckCircleIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const CheckIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ChevronDownIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ChevronRightIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const CloudArrowUpIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ComputerDesktopIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const CursorArrowRaysIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const Dialog: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const DialogPanel: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const Disclosure: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const DisclosureButton: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const DisclosurePanel: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const EnvelopeIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const FingerPrintIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const Fragment: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const HandRaisedIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const HeartIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const InboxIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const InformationCircleIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const LifebuoyIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const LockClosedIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const MinusIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const MinusSmallIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const NewspaperIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PencilSquareIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PhoneIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PlayCircleIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PlusIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PlusSmallIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const Popover: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PopoverButton: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PopoverGroup: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const PopoverPanel: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const QueueListIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const RectangleGroupIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const RssIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const ServerIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const SquaresPlusIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const StarIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const Tab: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const TabGroup: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const TabList: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const TabPanel: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const TabPanels: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const TrashIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const UsersIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const XMarkIcon: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const XMarkIconMini: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const XMarkIconOutline: (props?: Record<string, unknown>) => import("solid-js").JSX.Element;
  const day: { events: unknown[] };
  const tiers: unknown[];
}

export {};
