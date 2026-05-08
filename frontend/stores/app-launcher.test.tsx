import { beforeEach, describe, expect, it, vi } from "vitest"
import { cleanTestStorage, useTestStorageEngine as enableTestStorageEngine } from "@nanostores/persistent"

vi.mock("@/platform/runtime/app-manager", () => ({
  createAppLaunchPlan: () => ({
    runtime: "builtin",
    reason: "test",
    component: "mock app",
  }),
}))

enableTestStorageEngine()

const { launchAppById } = await import("./app-launcher")
const { appSurfacesStore, appSurfaceOrderStore, focusedAppSurfaceStore, pendingGateStore } = await import("./desktop-store")
const { installedAppIdsStore, installApp } = await import("./installed-apps-store")
const { localCapabilityGrantsStore } = await import("./local-capability-grants-store")

describe("app launcher", () => {
  beforeEach(() => {
    cleanTestStorage()
    appSurfacesStore.set([])
    appSurfaceOrderStore.set([])
    focusedAppSurfaceStore.set(null)
    pendingGateStore.set(null)
    localCapabilityGrantsStore.set({})
    installedAppIdsStore.set(["app-store", "settings"])
  })

  it("opens a single overlay at a time", () => {
    const first = launchAppById("app-store")
    expect(first).not.toBeNull()
    expect(appSurfacesStore.get()).toHaveLength(1)
    expect(appSurfacesStore.get()[0].appId).toBe("app-store")

    const second = launchAppById("settings")
    expect(second).not.toBeNull()
    expect(appSurfacesStore.get()).toHaveLength(1)
    expect(appSurfacesStore.get()[0].appId).toBe("settings")
    expect(appSurfaceOrderStore.get()).toEqual([appSurfacesStore.get()[0].id])
    expect(focusedAppSurfaceStore.get()).toBe(appSurfacesStore.get()[0].id)
  })

  it("keeps the current overlay open when a launch needs permission", () => {
    launchAppById("app-store")
    const currentSurface = appSurfacesStore.get()[0]
    installApp("google-drive")

    const blocked = launchAppById("google-drive")

    expect(blocked).toBeNull()
    expect(appSurfacesStore.get()).toEqual([currentSurface])
    expect(pendingGateStore.get()?.app.appId).toBe("google-drive")
    expect(pendingGateStore.get()?.blocked).toEqual(["identity", "filesystem"])
  })
})
