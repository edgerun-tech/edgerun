// SPDX-License-Identifier: Apache-2.0
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const publicFontsDir = path.join(root, 'public', 'fonts')

const weights = [400, 500, 700]

const sansFiles = weights.map((weight) => ({
  source: path.join(root, 'node_modules', '@fontsource', 'geist-sans', 'files', `geist-sans-latin-${weight}-normal.woff2`),
  target: `geist-sans-latin-${weight}.woff2`,
  family: 'Geist',
  weight
}))

const monoFiles = weights.map((weight) => ({
  source: path.join(root, 'node_modules', '@fontsource', 'geist-mono', 'files', `geist-mono-latin-${weight}-normal.woff2`),
  target: `geist-mono-latin-${weight}.woff2`,
  family: 'Geist Mono',
  weight
}))

const files = [...sansFiles, ...monoFiles]

rmSync(publicFontsDir, { recursive: true, force: true })
mkdirSync(publicFontsDir, { recursive: true })

for (const file of files) {
  copyFileSync(file.source, path.join(publicFontsDir, file.target))
}

const css = files
  .map(
    (file) => `@font-face {
  font-family: "${file.family}";
  font-style: normal;
  font-weight: ${file.weight};
  font-display: swap;
  src: url("/fonts/${file.target}") format("woff2");
}`
  )
  .join('\n\n')

writeFileSync(path.join(publicFontsDir, 'fonts.css'), `${css}\n`, 'utf8')
console.log(`Synced ${files.length} font files to public/fonts`)
