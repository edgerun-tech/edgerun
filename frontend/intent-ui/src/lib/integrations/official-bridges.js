const OFFICIAL_BRIDGES = [
  { id: "telegram", aliases: ["telegram"], repository: "mautrix/telegram" },
  { id: "whatsapp", aliases: ["whatsapp"], repository: "mautrix/whatsapp" },
  { id: "signal", aliases: ["signal"], repository: "mautrix/signal" },
  { id: "discord", aliases: ["discord"], repository: "mautrix/discord" },
  { id: "slack", aliases: ["slack"], repository: "mautrix/slack" },
  { id: "google_messages", aliases: ["gmessages", "googlemessages", "google_messages", "rcs", "sms"], repository: "mautrix/gmessages" },
  { id: "gvoice", aliases: ["gvoice", "googlevoice"], repository: "mautrix/gvoice" },
  { id: "meta", aliases: ["meta", "instagram", "facebook", "messenger"], repository: "mautrix/meta" },
  { id: "googlechat", aliases: ["googlechat", "gchat"], repository: "mautrix/googlechat" },
  { id: "twitter", aliases: ["twitter"], repository: "mautrix/twitter" },
  { id: "bluesky", aliases: ["bluesky", "bsky"], repository: "mautrix/bluesky" },
  { id: "imessage", aliases: ["imessage"], repository: "mautrix/imessage" },
  { id: "imessagego", aliases: ["imessagego"], repository: "beeper/imessage" },
  { id: "linkedin", aliases: ["linkedin"], repository: "mautrix/linkedin" },
  { id: "heisenbridge", aliases: ["heisenbridge", "irc"], repository: "hifi/heisenbridge" }
];

const aliasToCanonical = new Map();
for (const bridge of OFFICIAL_BRIDGES) {
  aliasToCanonical.set(bridge.id, bridge.id);
  for (const alias of bridge.aliases || []) {
    aliasToCanonical.set(alias, bridge.id);
  }
}

function normalizeBridgeIdentifier(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[\s-]+/g, "_");
}

export function canonicalBridgeId(value) {
  const normalized = normalizeBridgeIdentifier(value);
  return aliasToCanonical.get(normalized) || "";
}

export function isOfficialBridgeId(value) {
  return Boolean(canonicalBridgeId(value));
}

export {
  OFFICIAL_BRIDGES
};
