import { describe, expect, it } from "vitest"
import { getAppSurfaceSpec } from "./app-surface-registry"

describe("app surface registry", () => {
  it("uses semantic variants instead of pixel dimensions", () => {
    expect(getAppSurfaceSpec("calculator")).toMatchObject({ kind: "overlay", variant: "compact" })
    expect(getAppSurfaceSpec("app-store")).toMatchObject({ kind: "overlay", variant: "wide" })
    expect(getAppSurfaceSpec("storage")).toMatchObject({ kind: "overlay", variant: "full" })
  })

  it("falls back to a standard overlay for unknown apps", () => {
    expect(getAppSurfaceSpec("unknown-app")).toMatchObject({
      kind: "overlay",
      variant: "standard",
      dismissOnOutsideClick: true,
    })
  })
})
