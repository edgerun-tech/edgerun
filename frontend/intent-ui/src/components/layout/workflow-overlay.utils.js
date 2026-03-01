import { FiCloud, FiCpu, FiDatabase } from "solid-icons/fi";
import { SiGithub, SiGoogle, SiCloudflare, SiVercel, SiTelegram, SiWhatsapp, SiMessenger, SiTailscale, SiWeb3dotjs } from "solid-icons/si";

export function channelBadgeClass(channel) {
  if (channel === "ai") return "border-[hsl(var(--primary)/0.42)] bg-[hsl(var(--primary)/0.14)] text-[hsl(var(--primary))]";
  if (channel === "email") return "border-neutral-700 bg-neutral-800/80 text-neutral-300";
  if (channel === "contact") return "border-neutral-700 bg-neutral-800/70 text-neutral-400";
  return "border-neutral-700 bg-neutral-800/70 text-neutral-300";
}

export function parseEmailAddress(value) {
  const raw = String(value || "").trim();
  const match = raw.match(/<([^>]+)>/);
  if (match) return match[1].trim().toLowerCase();
  const emailMatch = raw.match(/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/i);
  return emailMatch ? emailMatch[0].toLowerCase() : raw.toLowerCase();
}

export function parseEmailName(value) {
  const raw = String(value || "").trim();
  const stripped = raw.replace(/<[^>]+>/g, "").replace(/\"/g, "").trim();
  if (stripped) return stripped;
  return parseEmailAddress(raw);
}

const INTEGRATION_ICON_MAP = {
  github: SiGithub,
  cloudflare: SiCloudflare,
  vercel: SiVercel,
  google: SiGoogle,
  google_photos: SiGoogle,
  email: SiGoogle,
  whatsapp: SiWhatsapp,
  signal: SiMessenger,
  discord: SiMessenger,
  slack: SiMessenger,
  messenger: SiMessenger,
  telegram: SiTelegram,
  google_messages: SiGoogle,
  gmessages: SiGoogle,
  googlemessages: SiGoogle,
  rcs: SiGoogle,
  sms: SiGoogle,
  gvoice: SiGoogle,
  googlevoice: SiGoogle,
  meta: SiMessenger,
  facebook: SiMessenger,
  instagram: SiMessenger,
  googlechat: SiGoogle,
  gchat: SiGoogle,
  twitter: SiMessenger,
  bluesky: SiMessenger,
  bsky: SiMessenger,
  imessage: SiMessenger,
  imessagego: SiMessenger,
  linkedin: SiMessenger,
  heisenbridge: FiDatabase,
  irc: FiDatabase,
  opencode: FiCpu,
  opencode_cli: FiCpu,
  tailscale: SiTailscale,
  hetzner: FiDatabase,
  web3: SiWeb3dotjs
};

export function integrationIconForSuggestion(integrationId) {
  return INTEGRATION_ICON_MAP[integrationId] || FiCloud;
}

export function groupFleetDevices(devices, currentDeviceId) {
  const grouped = new Map();
  for (const device of devices) {
    const isHost = device.type === "host";
    const isLocalBrowser = device.id === currentDeviceId;
    const key = isHost
      ? `host:${device.metadata?.host || device.ip || device.id}`
      : isLocalBrowser
        ? "local-browser"
        : `device:${device.id}`;
    if (!grouped.has(key)) {
      grouped.set(key, {
        id: key,
        name: isHost ? "Connected Host" : isLocalBrowser ? "This Device" : (device.name || device.id),
        type: isHost ? "host" : isLocalBrowser ? "local" : (device.type || "device"),
        online: Boolean(device.online),
        primary: device,
        members: [device],
        localAvailable: isLocalBrowser && Boolean(device.online)
      });
      continue;
    }
    const entry = grouped.get(key);
    entry.members.push(device);
    entry.online = entry.online || Boolean(device.online);
    entry.localAvailable = entry.localAvailable || (device.id === currentDeviceId && Boolean(device.online));
    const prevSeen = new Date(entry.primary?.lastSeenAt || 0).getTime();
    const nextSeen = new Date(device.lastSeenAt || 0).getTime();
    const preferHost = entry.primary?.type !== "host" && device.type === "host";
    if (preferHost || nextSeen > prevSeen) entry.primary = device;
  }
  return Array.from(grouped.values()).sort((a, b) => {
    if (a.online !== b.online) return a.online ? -1 : 1;
    const aSeen = new Date(a.primary?.lastSeenAt || 0).getTime();
    const bSeen = new Date(b.primary?.lastSeenAt || 0).getTime();
    return bSeen - aSeen;
  });
}
