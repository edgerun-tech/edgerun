/**
 * @typedef {object} IntegrationConfig
 * @property {string} id
 * @property {string} name
 * @property {string} description
 */

/** @type {Record<string, IntegrationConfig>} */
const integrations = {
  editor: { id: "editor", name: "Editor", description: "Code editor view" },
  files: { id: "files", name: "Files", description: "File manager panel" },
  integrations: { id: "integrations", name: "Integrations", description: "Connected services" },
  github: { id: "github", name: "GitHub", description: "OIDC app connection for repository access" },
  email: { id: "email", name: "Gmail", description: "Email management" },
  settings: { id: "settings", name: "Settings", description: "Application settings" },
  onvif: { id: "onvif", name: "ONVIF Cameras", description: "Camera endpoints and live previews" },
  terminal: { id: "terminal", name: "Terminal", description: "Terminal emulator" },
  cloud: { id: "cloud", name: "Cloud", description: "Cloud resources" },
  credentials: { id: "credentials", name: "Credentials", description: "hwvault credentials manager" },
  web3: { id: "web3", name: "Web3", description: "Wallet-backed encryption and key workflows" },
  theme: { id: "theme", name: "Theme", description: "Theme controls" }
};
/**
 * @param {string} id
 * @returns {IntegrationConfig | undefined}
 */
function getIntegrationById(id) {
  return integrations[id];
}
export {
  getIntegrationById
};
