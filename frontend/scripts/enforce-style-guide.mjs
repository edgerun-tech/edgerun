import { readFileSync } from 'node:fs'
import { execSync } from 'node:child_process'

const files = execSync("find app components -type f \\( -name '*.ts' -o -name '*.tsx' \\)", { encoding: 'utf8' })
  .split('\n')
  .map((s) => s.trim())
  .filter(Boolean)

const hexPattern = /#[0-9a-fA-F]{3,8}\b/g
const themedPrimitives = new Set([
  'components/ui/button.tsx',
  'components/ui/badge.tsx',
  'components/ui/card.tsx',
  'components/ui/input.tsx'
])
const bad = []

for (const file of files) {
  if (file.startsWith('app/style-guide/') || file.startsWith('components/style-guide/')) {
    continue
  }
  const src = readFileSync(file, 'utf8')
  if (src.includes('style=')) {
    bad.push(`${file}: inline style is not allowed in app/components`)
  }
  const matches = src.match(hexPattern)
  if (matches) {
    bad.push(`${file}: raw hex colors are not allowed (${Array.from(new Set(matches)).join(', ')})`)
  }

  if (themedPrimitives.has(file) && !src.includes('uiTheme')) {
    bad.push(`${file}: themed primitives must import and use uiTheme from lib/ui-theme.ts`)
  }
}

if (bad.length) {
  console.error('Style guide enforcement failed:')
  for (const line of bad) console.error(`- ${line}`)
  process.exit(1)
}

const globalStyles = readFileSync('src/styles.css', 'utf8')
if (!globalStyles.includes('font-family: "Geist"')) {
  console.error('Style guide enforcement failed: src/styles.css must define Geist as the primary UI font family')
  process.exit(1)
}

const buildFile = readFileSync('build.tsx', 'utf8')
if (!buildFile.includes('href="/fonts/fonts.css"')) {
  console.error('Style guide enforcement failed: build.tsx must load local /fonts/fonts.css')
  process.exit(1)
}
if (buildFile.includes('fonts.googleapis.com') || buildFile.includes('cdn.jsdelivr.net')) {
  console.error('Style guide enforcement failed: third-party runtime font URLs are not allowed in build.tsx')
  process.exit(1)
}

console.log('style-guide:check passed')
