/**
 * Canonical registry exports.
 *
 * New code should import registries from here unless it needs a private helper
 * from a specific registry module.
 */

export {
  BUILTIN_APPS,
  BUILTIN_ICON_MAP,
  getBuiltinApp,
  getIconById,
  listBuiltinApps,
  normalizeBuiltinAppId,
} from "./builtin-app-registry"

export {
  getAppSurfaceSpec,
  getDefaultSurfaceSize,
  registerAppSurfaceSpec,
  enrichAppWithSurfaceSpec,
  type AppSurfaceSpec,
} from "./app-surface-registry"

export {
  componentRegistry,
  registerComponent,
  getComponent,
  renderViewSpec,
  renderComponentTree,
  registerViewSpecRenderer,
  listRegisteredComponents,
  isComponentSafe,
  type ViewSpec,
  type ComponentSpec,
  type RegisteredComponent,
  type ComponentRegistryState,
} from "./component-registry"
