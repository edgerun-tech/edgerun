import { OIDC_SCOPES } from "./oidc-scopes";

const WINDOW_SCOPE_REQUIREMENTS = {
  credentials: {
    requiredAll: [OIDC_SCOPES.controlPlane.profileRead, OIDC_SCOPES.controlPlane.profileWrite]
  },
  integrations: {
    requiredAll: [OIDC_SCOPES.controlPlane.profileRead, OIDC_SCOPES.controlPlane.profileWrite]
  },
  github: {
    requiredAll: [OIDC_SCOPES.controlPlane.intentsSubmit, OIDC_SCOPES.capability.networkUse]
  },
  email: {
    requiredAll: [OIDC_SCOPES.controlPlane.intentsSubmit, OIDC_SCOPES.capability.networkUse]
  },
  drive: {
    requiredAll: [OIDC_SCOPES.capability.storageRead, OIDC_SCOPES.capability.storageWrite]
  },
  calendar: {
    requiredAll: [OIDC_SCOPES.controlPlane.intentsSubmit]
  },
  cloudflare: {
    requiredAll: [OIDC_SCOPES.controlPlane.intentsSubmit, OIDC_SCOPES.capability.networkUse]
  },
  cloud: {
    requiredAll: [OIDC_SCOPES.controlPlane.intentsSubmit, OIDC_SCOPES.capability.networkUse]
  },
  onvif: {
    requiredAll: [OIDC_SCOPES.capability.cameraUse, OIDC_SCOPES.capability.networkUse]
  }
};

export function requirementForWindow(windowId) {
  return WINDOW_SCOPE_REQUIREMENTS[String(windowId || "")] || null;
}

export function isProfileSensitiveWindow(windowId) {
  return requirementForWindow(windowId) !== null;
}
