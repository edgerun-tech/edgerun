export const OIDC_SCOPES = {
  identity: {
    openid: "openid",
    profile: "profile",
    email: "email"
  },
  controlPlane: {
    profileRead: "edgerun:profile.read",
    profileWrite: "edgerun:profile.write",
    nodesRead: "edgerun:nodes.read",
    nodesBind: "edgerun:nodes.bind",
    intentsSubmit: "edgerun:intents.submit",
    intentsApprove: "edgerun:intents.approve"
  },
  capability: {
    storageRead: "edgerun:cap.storage.read",
    storageWrite: "edgerun:cap.storage.write",
    networkUse: "edgerun:cap.network.use",
    usbUse: "edgerun:cap.usb.use",
    cameraUse: "edgerun:cap.camera.use",
    microphoneUse: "edgerun:cap.microphone.use",
    gpuUse: "edgerun:cap.gpu.use"
  },
  executor: {
    wasm: "edgerun:exec.wasm",
    oci: "edgerun:exec.oci",
    vm: "edgerun:exec.vm",
    lxc: "edgerun:exec.lxc"
  },
  admin: {
    policyWrite: "edgerun:policy.write",
    clusterAdmin: "edgerun:cluster.admin",
    nodeRecover: "edgerun:node.recover"
  }
};

export const DEFAULT_LOCAL_PROFILE_SCOPES = [
  OIDC_SCOPES.identity.openid,
  OIDC_SCOPES.identity.profile,
  OIDC_SCOPES.controlPlane.profileRead,
  OIDC_SCOPES.controlPlane.profileWrite,
  OIDC_SCOPES.controlPlane.nodesRead,
  OIDC_SCOPES.controlPlane.nodesBind,
  OIDC_SCOPES.controlPlane.intentsSubmit,
  OIDC_SCOPES.controlPlane.intentsApprove,
  OIDC_SCOPES.capability.storageRead,
  OIDC_SCOPES.capability.storageWrite,
  OIDC_SCOPES.capability.networkUse,
  OIDC_SCOPES.capability.usbUse,
  OIDC_SCOPES.capability.cameraUse,
  OIDC_SCOPES.capability.microphoneUse,
  OIDC_SCOPES.capability.gpuUse,
  OIDC_SCOPES.executor.wasm,
  OIDC_SCOPES.executor.oci,
  OIDC_SCOPES.executor.vm,
  OIDC_SCOPES.executor.lxc
];

const uniq = (items) => Array.from(new Set((items || []).filter(Boolean)));

export const normalizeScopes = (items) => uniq((items || []).map((value) => String(value || "").trim()));

export function scopeRequirementSatisfied(grantedScopes, requirement) {
  const granted = new Set(normalizeScopes(grantedScopes));
  const requiredAll = normalizeScopes(requirement?.requiredAll || requirement?.required_all || []);
  const requiredAny = normalizeScopes(requirement?.requiredAny || requirement?.required_any || []);

  if (requiredAll.some((scope) => !granted.has(scope))) return false;
  if (requiredAny.length > 0 && !requiredAny.some((scope) => granted.has(scope))) return false;
  return true;
}
