const OFFICIAL_BRIDGES = [
  {
    id: "beeper",
    aliases: [
      "beeper",
      "telegram",
      "whatsapp",
      "signal",
      "discord",
      "slack",
      "gmessages",
      "googlemessages",
      "google_messages",
      "rcs",
      "sms",
      "gvoice",
      "googlevoice",
      "meta",
      "instagram",
      "facebook",
      "messenger",
      "googlechat",
      "gchat",
      "twitter",
      "bluesky",
      "bsky",
      "imessage",
      "imessagego",
      "linkedin",
      "heisenbridge",
      "irc"
    ],
    repository: "beeper/desktop-api"
  }
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
