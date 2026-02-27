import type { JSX } from "solid-js";
import { Icon } from "solid-heroicons";
import * as miniIcons from "solid-heroicons/solid-mini";
import * as outlineIcons from "solid-heroicons/outline";
import * as solidIcons from "solid-heroicons/solid";

import {
  Disclosure,
  DisclosureButton,
  DisclosurePanel,
  Popover,
  PopoverButton,
  PopoverGroup,
  PopoverPanel,
} from "./index";

export {
  Disclosure,
  DisclosureButton,
  DisclosurePanel,
  Popover,
  PopoverButton,
  PopoverGroup,
  PopoverPanel,
};

type AnyProps = Record<string, unknown> & { children?: JSX.Element };
type HeroiconDef = { path: JSX.Element; outline: boolean; mini: boolean };

const outlineLookup = outlineIcons as unknown as Record<string, HeroiconDef>;
const solidLookup = solidIcons as unknown as Record<string, HeroiconDef>;
const miniLookup = miniIcons as unknown as Record<string, HeroiconDef>;

const keyOverrides: Record<string, string> = {
  Bars3Icon: "bars_3",
  BuildingOffice2Icon: "buildingOffice_2",
  Cog6ToothIcon: "cog_6Tooth",
  MinusSmallIcon: "minusSmall",
  PlusSmallIcon: "plusSmall",
  XMarkIcon: "xMark",
  XMarkIconMini: "xMark",
  XMarkIconOutline: "xMark",
};

const toHeroiconKey = (name: string) => {
  const overridden = keyOverrides[name];
  if (overridden) return overridden;

  const base = name.replace(/Icon(?:Mini|Outline)?$/, "");
  const camel = `${base.slice(0, 1).toLowerCase()}${base.slice(1)}`;
  return camel.replace(/([A-Za-z])(\d)/g, "$1_$2");
};

const resolveIcon = (name: string): HeroiconDef | undefined => {
  const key = toHeroiconKey(name);
  const preferMini = name.endsWith("Mini") || name.includes("Small");
  const lookups = preferMini
    ? [miniLookup, solidLookup, outlineLookup]
    : [outlineLookup, solidLookup, miniLookup];

  for (const table of lookups) {
    if (table[key]) return table[key];
  }

  return undefined;
};

const passthrough = (props: AnyProps) => <>{props.children}</>;
const block = (props: AnyProps) => (
  <div {...(props as JSX.HTMLAttributes<HTMLDivElement>)}>{props.children}</div>
);
const Fragment = passthrough;

const fallbackIcon = (props: AnyProps) => (
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    aria-hidden="true"
    {...(props as JSX.SvgSVGAttributes<SVGSVGElement>)}
  >
    <path d="M4 12h16M12 4v16" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

const createIcon = (name: string) => {
  const path = resolveIcon(name);
  if (!path) return fallbackIcon;

  return (props: AnyProps) => (
    <Icon path={path} aria-hidden="true" {...(props as JSX.SvgSVGAttributes<SVGSVGElement>)} />
  );
};

export const Dialog = block;
export const DialogPanel = block;
export const TabGroup = block;
export const TabList = block;
export const TabPanels = block;
export const TabPanel = block;
export const Tab = passthrough;

export const ArrowPathIcon = createIcon("ArrowPathIcon");
export const AcademicCapIcon = createIcon("AcademicCapIcon");
export const Bars3Icon = createIcon("Bars3Icon");
export const BriefcaseIcon = createIcon("BriefcaseIcon");
export const BoltIcon = createIcon("BoltIcon");
export const BookOpenIcon = createIcon("BookOpenIcon");
export const BookmarkSquareIcon = createIcon("BookmarkSquareIcon");
export const BugAntIcon = createIcon("BugAntIcon");
export const BuildingOffice2Icon = createIcon("BuildingOffice2Icon");
export const CalendarDaysIcon = createIcon("CalendarDaysIcon");
export const ChartPieIcon = createIcon("ChartPieIcon");
export const ChatBubbleLeftRightIcon = createIcon("ChatBubbleLeftRightIcon");
export const ChatBubbleOvalLeftEllipsisIcon = createIcon("ChatBubbleOvalLeftEllipsisIcon");
export const CheckCircleIcon = createIcon("CheckCircleIcon");
export const CheckIcon = createIcon("CheckIcon");
export const ChevronDownIcon = createIcon("ChevronDownIcon");
export const ChevronRightIcon = createIcon("ChevronRightIcon");
export const Cog6ToothIcon = createIcon("Cog6ToothIcon");
export const CloudArrowUpIcon = createIcon("CloudArrowUpIcon");
export const ComputerDesktopIcon = createIcon("ComputerDesktopIcon");
export const CursorArrowRaysIcon = createIcon("CursorArrowRaysIcon");
export const DocumentChartBarIcon = createIcon("DocumentChartBarIcon");
export const EnvelopeIcon = createIcon("EnvelopeIcon");
export const FingerPrintIcon = createIcon("FingerPrintIcon");
export const GlobeAltIcon = createIcon("GlobeAltIcon");
export const HandRaisedIcon = createIcon("HandRaisedIcon");
export const HeartIcon = createIcon("HeartIcon");
export const InboxIcon = createIcon("InboxIcon");
export const InformationCircleIcon = createIcon("InformationCircleIcon");
export const LifebuoyIcon = createIcon("LifebuoyIcon");
export const LockClosedIcon = createIcon("LockClosedIcon");
export const MinusIcon = createIcon("MinusIcon");
export const MinusSmallIcon = createIcon("MinusSmallIcon");
export const NewspaperIcon = createIcon("NewspaperIcon");
export const PencilSquareIcon = createIcon("PencilSquareIcon");
export const PhoneIcon = createIcon("PhoneIcon");
export const PlayCircleIcon = createIcon("PlayCircleIcon");
export const PlusIcon = createIcon("PlusIcon");
export const PlusSmallIcon = createIcon("PlusSmallIcon");
export const QueueListIcon = createIcon("QueueListIcon");
export const RectangleGroupIcon = createIcon("RectangleGroupIcon");
export const RocketLaunchIcon = createIcon("RocketLaunchIcon");
export const RssIcon = createIcon("RssIcon");
export const ServerIcon = createIcon("ServerIcon");
export const ShieldCheckIcon = createIcon("ShieldCheckIcon");
export const SparklesIcon = createIcon("SparklesIcon");
export const SquaresPlusIcon = createIcon("SquaresPlusIcon");
export const StarIcon = createIcon("StarIcon");
export const SunIcon = createIcon("SunIcon");
export const TrashIcon = createIcon("TrashIcon");
export const UserGroupIcon = createIcon("UserGroupIcon");
export const UsersIcon = createIcon("UsersIcon");
export const VideoCameraIcon = createIcon("VideoCameraIcon");
export const XMarkIcon = createIcon("XMarkIcon");
export const XMarkIconMini = createIcon("XMarkIconMini");
export const XMarkIconOutline = createIcon("XMarkIconOutline");

export const day = { events: [] as unknown[] };
export const tiers = [] as unknown[];

const runtimeGlobals = {
  ArrowPathIcon,
  AcademicCapIcon,
  Bars3Icon,
  BriefcaseIcon,
  BoltIcon,
  BookOpenIcon,
  BookmarkSquareIcon,
  BugAntIcon,
  BuildingOffice2Icon,
  CalendarDaysIcon,
  ChartPieIcon,
  ChatBubbleLeftRightIcon,
  ChatBubbleOvalLeftEllipsisIcon,
  CheckCircleIcon,
  CheckIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  Cog6ToothIcon,
  CloudArrowUpIcon,
  ComputerDesktopIcon,
  CursorArrowRaysIcon,
  DocumentChartBarIcon,
  Dialog,
  DialogPanel,
  Disclosure,
  DisclosureButton,
  DisclosurePanel,
  EnvelopeIcon,
  FingerPrintIcon,
  Fragment,
  GlobeAltIcon,
  HandRaisedIcon,
  HeartIcon,
  InboxIcon,
  InformationCircleIcon,
  LifebuoyIcon,
  LockClosedIcon,
  MinusIcon,
  MinusSmallIcon,
  NewspaperIcon,
  PencilSquareIcon,
  PhoneIcon,
  PlayCircleIcon,
  PlusIcon,
  PlusSmallIcon,
  Popover,
  PopoverButton,
  PopoverGroup,
  PopoverPanel,
  QueueListIcon,
  RectangleGroupIcon,
  RocketLaunchIcon,
  RssIcon,
  ServerIcon,
  ShieldCheckIcon,
  SparklesIcon,
  SquaresPlusIcon,
  StarIcon,
  SunIcon,
  Tab,
  TabGroup,
  TabList,
  TabPanel,
  TabPanels,
  TrashIcon,
  UserGroupIcon,
  UsersIcon,
  VideoCameraIcon,
  XMarkIcon,
  XMarkIconMini,
  XMarkIconOutline,
  day,
  tiers,
};

Object.assign(globalThis as Record<string, unknown>, runtimeGlobals);

export {};
