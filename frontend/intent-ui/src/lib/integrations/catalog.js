import { IntegrationLifecycle } from "./IntegrationLifecycle";

const integrationDefinitions = [
  {
    id: "github",
    name: "GitHub",
    authMethod: "token",
    supportsPlatformConnector: false,
    defaultConnectorMode: "user_owned",
    tokenKey: "github_token",
    defaultCapabilities: ["repos.read", "repos.write", "prs.read", "prs.write"],
    tags: ["code", "storage", "workflows"]
  },
  {
    id: "cloudflare",
    name: "Cloudflare",
    authMethod: "token",
    supportsPlatformConnector: true,
    defaultConnectorMode: "user_owned",
    tokenKey: "cloudflare_token",
    defaultCapabilities: ["zones.read", "workers.read", "workers.write"],
    tags: ["network", "deploy", "workflows", "code"]
  },
  {
    id: "vercel",
    name: "Vercel",
    authMethod: "token",
    supportsPlatformConnector: true,
    tokenKey: "vercel_token",
    defaultCapabilities: ["projects.read", "deployments.read", "deployments.write"],
    tags: ["deploy", "workflows", "code"]
  },
  {
    id: "google",
    name: "Google",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "google_token",
    defaultCapabilities: ["drive.read", "gmail.read", "calendar.read", "contacts.read", "messages.read"],
    tags: ["messages", "storage", "workflows", "productivity"]
  },
  {
    id: "google_photos",
    name: "Google Photos",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "google_token",
    defaultCapabilities: ["photos.read"],
    tags: ["media", "storage"]
  },
  {
    id: "email",
    name: "Email",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "google_token",
    defaultCapabilities: ["messages.read", "messages.send"],
    tags: ["messages", "workflows"]
  },
  {
    id: "beeper",
    name: "Beeper",
    authMethod: "token",
    supportsPlatformConnector: false,
    defaultConnectorMode: "user_owned",
    forceUserOwned: true,
    tokenKey: "beeper_access_token",
    defaultCapabilities: ["messages.read", "messages.send", "messages.search", "accounts.read"],
    tags: ["messages", "workflows", "beeper-desktop"],
    aliases: [
      "beeper",
      "telegram",
      "whatsapp",
      "signal",
      "discord",
      "slack",
      "google_messages",
      "gmessages",
      "meta",
      "googlechat",
      "gvoice",
      "twitter",
      "bluesky",
      "imessage",
      "imessagego",
      "linkedin",
      "heisenbridge",
      "irc",
      "messenger"
    ]
  },
  {
    id: "opencode_cli",
    name: "OpenCode CLI",
    authMethod: "local_cli",
    supportsPlatformConnector: false,
    requiresToken: false,
    tokenKey: "",
    defaultCapabilities: ["assistant.local_cli.execute"],
    tags: ["ai", "code", "workflows"]
  },
  {
    id: "tailscale",
    name: "Tailscale",
    authMethod: "token",
    supportsPlatformConnector: true,
    defaultConnectorMode: "user_owned",
    tokenKey: "tailscale_api_key",
    defaultCapabilities: ["network.overlay.join", "network.overlay.funnel", "network.overlay.ssh"],
    tags: ["network", "devices", "workflows"]
  },
  {
    id: "hetzner",
    name: "Hetzner",
    authMethod: "token",
    supportsPlatformConnector: true,
    tokenKey: "hetzner_token",
    defaultCapabilities: ["servers.read", "servers.write", "firewalls.read"],
    tags: ["compute", "network", "storage", "workflows"]
  },
  {
    id: "web3",
    name: "Web3",
    authMethod: "wallet",
    supportsPlatformConnector: false,
    tokenKey: "web3_wallet",
    defaultCapabilities: ["wallet.connect", "profile.encrypt", "backup.local"],
    tags: ["identity", "security", "workflows"]
  },
  {
    id: "flipper",
    name: "Flipper",
    authMethod: "web_bluetooth",
    supportsPlatformConnector: false,
    requiresToken: false,
    tokenKey: "flipper_device_id",
    defaultCapabilities: ["bluetooth.connect", "bluetooth.gatt", "hardware.flipper.interact"],
    tags: ["devices", "security", "workflows"]
  },
  {
    id: "daly_bms",
    name: "Daly BMS",
    authMethod: "web_bluetooth",
    supportsPlatformConnector: false,
    requiresToken: false,
    tokenKey: "daly_bms_device_id",
    defaultCapabilities: ["bluetooth.connect", "bluetooth.gatt", "hardware.bms.read"],
    tags: ["devices", "energy", "workflows"]
  }
];

function createIntegrationCatalog() {
  return Object.fromEntries(
    integrationDefinitions.map((definition) => [definition.id, new IntegrationLifecycle(definition)])
  );
}

export {
  integrationDefinitions,
  createIntegrationCatalog
};
