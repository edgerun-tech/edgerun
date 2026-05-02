import { atom } from "nanostores"
/**
 * Single registry for dashboard-renderable UI components.
 * Safe declarative UI component mapping, shadcn presets, ViewSpec support.
 * No arbitrary dynamic imports for untrusted apps in MVP.
 */

export interface ViewSpec {
  type: "react" | "html" | "canvas"
  componentTree?: ComponentSpec[]
  styles?: Record<string, string>
}

export interface ComponentSpec {
  type: string
  props?: Record<string, unknown>
  children?: ComponentSpec[]
}

export interface RegisteredComponent {
  type: string
  component: React.ComponentType<unknown>
  isSafe: boolean
  allowedProps: string[]
  description?: string
}

export interface ComponentRegistryState {
  components: Map<string, RegisteredComponent>
  viewSpecRenderers: Map<string, (spec: ViewSpec) => React.ReactNode>
}

const initialState: ComponentRegistryState = {
  components: new Map(),
  viewSpecRenderers: new Map(),
}

export const componentRegistry = atom<ComponentRegistryState>(initialState)

let registryState: ComponentRegistryState = { ...initialState }

export function registerComponent(def: RegisteredComponent): void {
  registryState = {
    ...registryState,
    components: new Map(registryState.components).set(def.type, def),
  }
  componentRegistry.set({ ...registryState })
}

export function getComponent(type: string): RegisteredComponent | undefined {
  return registryState.components.get(type)
}

export function renderViewSpec(
  spec: ViewSpec,
): React.ReactNode {
  const renderer = registryState.viewSpecRenderers.get(spec.type)
  if (renderer) {
    return renderer(spec)
  }

  if (spec.componentTree) {
    return renderComponentTree(spec.componentTree)
  }

  return null
}

export function renderComponentTree(
  components: ComponentSpec[],
): React.ReactNode[] {
  return components.map((spec, index) => {
    const registered = getComponent(spec.type)
    if (!registered) {
      return null
    }

    if (!registered.isSafe) {
      console.warn(`Component ${spec.type} is not safe for untrusted rendering`)
      return null
    }

    const childNodes = spec.children
      ? renderComponentTree(spec.children)
      : undefined

    return (
      <registered.component
        key={`${spec.type}-${index}`}
        {...(spec.props as Record<string, unknown>)}
      >
        {childNodes}
      </registered.component>
    )
  })
}

export function registerViewSpecRenderer(
  type: string,
  renderer: (spec: ViewSpec) => React.ReactNode,
): void {
  registryState = {
    ...registryState,
    viewSpecRenderers: new Map(registryState.viewSpecRenderers).set(
      type,
      renderer,
    ),
  }
}

export function listRegisteredComponents(): RegisteredComponent[] {
  return Array.from(registryState.components.values())
}

export function isComponentSafe(type: string): boolean {
  return getComponent(type)?.isSafe ?? false
}
