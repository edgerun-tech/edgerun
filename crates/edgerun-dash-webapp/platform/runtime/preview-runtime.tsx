/**
 * Preview runtime for App Studio.
 * Handles preview mode, component rendering, pipeline preview.
 */

import { atom, computed } from "nanostores"
import { componentRegistry, getComponent } from "@/platform/registries/component-registry"

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

export interface PreviewState {
  isPreviewing: boolean
  currentViewSpec?: ViewSpec
  previewProps: Record<string, unknown>
  logs: string[]
}

const initialState: PreviewState = {
  isPreviewing: false,
  previewProps: {},
  logs: [],
}

export const previewRuntime = atom<PreviewState>(initialState)

export function startPreview(viewSpec: ViewSpec): void {
  previewRuntime.set({
    isPreviewing: true,
    currentViewSpec: viewSpec,
    previewProps: {},
    logs: [`Preview started at ${new Date().toISOString()}`],
  })
}

export function stopPreview(): void {
  const state = previewRuntime.get()
  previewRuntime.set({
    ...state,
    isPreviewing: false,
    currentViewSpec: undefined,
  })
}

export function updatePreviewProps(
  props: Record<string, unknown>,
): void {
  const state = previewRuntime.get()
  previewRuntime.set({
    ...state,
    previewProps: { ...state.previewProps, ...props },
  })
}

export function renderPreview(): React.ReactNode {
  const state = previewRuntime.get()
  if (!state.currentViewSpec) return null

  if (state.currentViewSpec.componentTree) {
    return renderComponentTree(state.currentViewSpec.componentTree)
  }

  return null
}

function renderComponentTree(
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

    const Component = registered.component as React.ComponentType<{ children?: React.ReactNode }>

    return (
      <Component
        key={`${spec.type}-${index}`}
        {...spec.props}
      >
        {childNodes}
      </Component>
    )
  })
}

export function addPreviewLog(message: string): void {
  const state = previewRuntime.get()
  previewRuntime.set({
    ...state,
    logs: [...state.logs, message].slice(-100),
  })
}
