import { spawnSync } from "node:child_process"
import { mkdtemp, readFile, readdir, rename, rm, stat } from "node:fs/promises"
import { join } from "node:path"

const catalogSourcePath = "public/apps/catalog.json"
const rkyvCatalogPath = "public/apps/catalog.ecat"
const appsRootPath = "public/apps"
const storeSeed = normalizeHex(process.env.EDGERUN_APP_STORE_SEED, "EDGERUN_APP_STORE_SEED")
const expectedStoreId = normalizeHex(process.env.NEXT_PUBLIC_EDGERUN_APP_STORE_ID, "NEXT_PUBLIC_EDGERUN_APP_STORE_ID")

function normalizeHex(value, name) {
  const normalized = value?.trim().toLowerCase()
  if (!normalized || !/^[0-9a-f]{64}$/.test(normalized)) {
    throw new Error(`${name} must be set to 32 hex bytes`)
  }
  return normalized
}

async function preflightCatalogSource() {
  const source = JSON.parse(await readFile(catalogSourcePath, "utf8"))
  if (source?.format !== "edgerun-app-catalog-source-v1" || !Array.isArray(source.apps)) {
    throw new Error(`${catalogSourcePath} must be an edgerun-app-catalog-source-v1 object`)
  }
  if (source.apps.length === 0) {
    throw new Error(`${catalogSourcePath} must list at least one app`)
  }

  const seen = new Set()
  for (const [index, app] of source.apps.entries()) {
    const slug = app?.slug
    if (typeof slug !== "string" || !/^[A-Za-z0-9._-]+$/.test(slug)) {
      throw new Error(`${catalogSourcePath} apps[${index}].slug is invalid`)
    }
    if (seen.has(slug)) {
      throw new Error(`${catalogSourcePath} lists duplicate app slug: ${slug}`)
    }
    seen.add(slug)
    await preflightPackagedApp(slug)
  }
  await preflightNoUnlistedPackages(seen)
}

async function preflightPackagedApp(slug) {
  const appDir = join(appsRootPath, slug)
  const appDirStat = await stat(appDir).catch(() => null)
  if (!appDirStat?.isDirectory()) {
    throw new Error(`catalog app is missing its package directory: ${appDir}`)
  }

  await stat(join(appDir, "app.eapp")).catch(() => {
    throw new Error(`catalog app is missing app.eapp: ${slug}`)
  })

  const manifestBytes = await readFile(join(appDir, "app.edapp")).catch(() => {
    throw new Error(`catalog app is missing app.edapp: ${slug}`)
  })
  const firstMeaningfulByte = manifestBytes.find((byte) => ![9, 10, 13, 32].includes(byte))
  if (firstMeaningfulByte === 0x7b || firstMeaningfulByte === 0x5b) {
    throw new Error(`catalog app uses a JSON app.edapp; repackage it as rkyv before publishing: ${slug}`)
  }
}

async function preflightNoUnlistedPackages(listedSlugs) {
  const entries = await readdir(appsRootPath, { withFileTypes: true })
  for (const entry of entries) {
    if (!entry.isDirectory() || listedSlugs.has(entry.name)) {
      continue
    }

    const appDir = join(appsRootPath, entry.name)
    const hasPackageArtifact = await stat(join(appDir, "app.eapp")).then(
      () => true,
      () => stat(join(appDir, "app.edapp")).then(() => true, () => false),
    )
    if (hasPackageArtifact) {
      throw new Error(`unlisted packaged app under ${appsRootPath}: ${entry.name}`)
    }
  }
}

async function refreshCatalog() {
  await preflightCatalogSource()

  const tempDir = await mkdtemp(`${rkyvCatalogPath}.tmp-`)
  const tempCatalogPath = join(tempDir, "catalog.ecat")
  try {
    const result = spawnSync(
      "cargo",
      [
        "run",
        "-p",
        "edgerun-sdk",
        "--",
        "write-app-store-catalog",
        `frontend/${tempCatalogPath}`,
        storeSeed,
        "1",
        "frontend/public/apps",
        `frontend/${catalogSourcePath}`,
      ],
      { cwd: "..", encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
    )
    if (result.stdout) {
      process.stdout.write(result.stdout)
    }
    if (result.status !== 0) {
      throw new Error(`failed to write rkyv app store catalog: exit ${result.status}`)
    }
    const storeId = result.stdout?.match(/^store_public_key: ([0-9a-f]{64})$/m)?.[1]
    if (!storeId) {
      throw new Error("edgerun-sdk did not report a store_public_key")
    }
    if (storeId !== expectedStoreId) {
      throw new Error("EDGERUN_APP_STORE_SEED does not match NEXT_PUBLIC_EDGERUN_APP_STORE_ID")
    }
    await rename(tempCatalogPath, rkyvCatalogPath)
  } finally {
    await rm(tempDir, { force: true, recursive: true })
  }
  console.log(`refreshed ${rkyvCatalogPath} from ${catalogSourcePath}`)
}

await refreshCatalog()
