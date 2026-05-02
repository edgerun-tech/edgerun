/**
 * Preview runtime for App Studio.
 * Handles preview mode, component rendering, pipeline preview.
 */

import { atom, computed } from "nanostores"
import type { ViewSpec, ComponentSpec } from "@/platform/protocol/apps"
import { componentRegistry } from "@/platform/registries/component-registry"

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
    const registered = componentRegistry.getComponent(spec.type)
    if (!registered || !registered.isSafe) {
      return null
    }

    const childNodes = spec.children
      ? renderComponentTree(spec.children)
      : undefined

    return (
      <registered.component
        key={`preview-${spec.type}-${index}`}
        {...(spec.props as Record<string, unknown>)}
      >
        {childNodes}
      </registered.component>
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
