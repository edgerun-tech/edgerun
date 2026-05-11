import type { AppDefinition, AppSignatureInfo } from "@/platform/types/app-definition"
import type { BrowserCatalogApp, InstalledBrowserApp } from "@/platform/runtime/browser-app-install-store"

export type AppAccessPolicyMode = "free-run" | "paid-run" | "paid-cache" | "license-required"

export interface VerifiedAppPackageProjection {
  appId: string
  name: string
  version: string
  description: string
  slug: string
  runtime: "browser-iframe"
  packageUrl: string
  manifestUrl: string
  launchUrl: string
  packageHash: string
  manifestHash: string
  packageBytes: number
  runtimeAppId?: string
  releaseId?: string
  developerId?: string
  developerName: string
  requiredCapabilityIds: string[]
  optionalCapabilityIds: string[]
  verified: boolean
  verifiedAt?: string
  verifiedAssets: number
  authorityRef: string
  proofRef: string
  installEventKind?: "app_package_verified"
  distributionModel: "network-storage-run"
  localCacheStatus: "not-cached" | "cached"
  appPolicyHash?: string
  accessPolicyMode: AppAccessPolicyMode
}

export function browserCatalogAppToPackageProjection(
  app: BrowserCatalogApp,
  installed?: InstalledBrowserApp,
): VerifiedAppPackageProjection {
  return {
    appId: app.appId,
    name: app.name,
    version: app.version,
    description: app.description,
    slug: app.slug,
    runtime: "browser-iframe",
    packageUrl: app.packageUrl,
    manifestUrl: app.manifestUrl,
    launchUrl: app.launchUrl,
    packageHash: app.eappSha256,
    manifestHash: app.manifestSha256,
    packageBytes: installed?.packageBytes ?? app.packageBytes,
    runtimeAppId: installed?.runtimeAppId || app.runtimeAppId,
    releaseId: installed?.releaseId || app.releaseId,
    developerId: installed?.developerId || app.developerId,
    developerName: installed?.developer || app.developer,
    requiredCapabilityIds: app.requiredCapabilityIds,
    optionalCapabilityIds: app.optionalCapabilityIds,
    verified: Boolean(installed),
    verifiedAt: installed?.installedAt,
    verifiedAssets: installed?.verifiedAssets.length ?? 0,
    authorityRef: "catalog:/apps/catalog.ecat",
    proofRef: app.packageUrl,
    installEventKind: installed ? "app_package_verified" : undefined,
    distributionModel: "network-storage-run",
    localCacheStatus: installed ? "cached" : "not-cached",
    appPolicyHash: undefined,
    accessPolicyMode: "free-run",
  }
}

export function packageProjectionSignatureInfo(pkg: VerifiedAppPackageProjection): AppSignatureInfo {
  return {
    developerId: pkg.developerId || pkg.developerName,
    developerName: pkg.developerName,
    packageHash: pkg.packageHash,
    manifestHash: pkg.manifestHash,
    verified: pkg.verified,
    verifiedAt: pkg.verifiedAt,
    authorityRef: pkg.authorityRef,
    proofRef: pkg.proofRef,
  }
}

export function packageProjectionToAppDefinition(pkg: VerifiedAppPackageProjection): AppDefinition {
  return {
    appId: pkg.appId,
    name: pkg.name,
    description: pkg.description,
    iconId: "wasm-generic",
    kind: "wasm",
    source: "catalog",
    requiredCapabilityIds: pkg.requiredCapabilityIds,
    optionalCapabilityIds: pkg.optionalCapabilityIds,
    status: pkg.verified ? "installed" : "available",
    signature: packageProjectionSignatureInfo(pkg),
    displayMetadata: {
      sourceOfTruth: "sdk-signed-content-addressed-package",
      distributionModel: pkg.distributionModel,
      localCacheStatus: pkg.localCacheStatus,
      appPolicyHash: pkg.appPolicyHash,
      accessPolicyMode: pkg.accessPolicyMode,
      runtimeManifest: {
        runtime: pkg.runtime,
        permissions: pkg.requiredCapabilityIds,
        optionalPermissions: pkg.optionalCapabilityIds,
      },
      runtime: pkg.runtime,
      packageUrl: pkg.packageUrl,
      manifestUrl: pkg.manifestUrl,
      launchUrl: pkg.launchUrl,
      eappSha256: pkg.packageHash,
      packageHash: pkg.packageHash,
      manifestSha256: pkg.manifestHash,
      manifestHash: pkg.manifestHash,
      packageBytes: pkg.packageBytes,
      runtimeAppId: pkg.runtimeAppId,
      releaseId: pkg.releaseId,
      developerId: pkg.developerId,
      developerName: pkg.developerName,
      installedAt: pkg.verifiedAt,
      verifiedAssets: pkg.verifiedAssets,
      installEventKind: pkg.installEventKind,
      authorityRef: pkg.authorityRef,
      proofRef: pkg.proofRef,
    },
  }
}
