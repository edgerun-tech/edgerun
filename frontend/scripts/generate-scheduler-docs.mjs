// SPDX-License-Identifier: Apache-2.0
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const frontendRoot = path.resolve(__dirname, '..')
const repoRoot = path.resolve(frontendRoot, '..')
const schedulerPath = path.join(repoRoot, 'crates/edgerun-scheduler/src/main.rs')
const outputDir = process.env.EDGERUN_FRONTEND_GENERATED_ROOT || path.join(repoRoot, 'out', 'frontend', 'generated')
const outputPath = path.join(outputDir, 'scheduler-api.json')

const source = readFileSync(schedulerPath, 'utf8')

function parseRoutes(src) {
  const routes = []
  const routeRegex = /\.route\("([^"]+)",\s*(get|post)\(([^)]+)\)\)/g
  for (const match of src.matchAll(routeRegex)) {
    routes.push({
      path: match[1],
      method: match[2].toUpperCase(),
      handler: match[3].trim()
    })
  }
  return routes
}

const routes = parseRoutes(source)
const generated = {
  generatedAt: new Date().toISOString(),
  sourceFile: 'crates/edgerun-scheduler/src/main.rs',
  sourceSha256: createHash('sha256').update(source).digest('hex'),
  endpointCount: routes.length,
  endpoints: routes
}

mkdirSync(outputDir, { recursive: true })
writeFileSync(outputPath, `${JSON.stringify(generated, null, 2)}\n`, 'utf8')
