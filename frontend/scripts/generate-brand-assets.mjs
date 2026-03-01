// SPDX-License-Identifier: Apache-2.0
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { execFileSync, spawnSync } from 'node:child_process'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const root = path.resolve(__dirname, '..')
const publicDir = path.join(root, 'public')
const brandDir = path.join(publicDir, 'brand')
const configPath = path.join(root, 'config', 'brand-theme.json')
const theme = JSON.parse(readFileSync(configPath, 'utf8'))
const lightBg = theme.colors.lightBackground || '#ffffff'
const darkBg = theme.colors.darkBackground || '#000000'
const lightFg = theme.colors.lightForeground || '#171717'
const darkFg = theme.colors.darkForeground || '#fafafa'
const brandPrimary = theme.colors.brandPrimary || '#7c3aed'
const neutralMarkFg = '#808080'

mkdirSync(brandDir, { recursive: true })

const markPath = `
M96 24 L168 48 L96 72 L24 48 Z
M24 72 L96 96 L168 72 L168 96 L96 120 L24 96 Z
M24 120 L96 144 L168 120 L168 144 L96 168 L24 144 Z
`.trim()

function markSvg(fg, bg = 'none') {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192" fill="none">
  ${bg === 'none' ? '' : `<rect width="192" height="192" rx="32" fill="${bg}" />`}
  <path d="${markPath}" fill="${fg}" />
</svg>
`
}

const logoHorizontalSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 720 170" fill="none">
  <rect width="720" height="170" fill="${lightBg}"/>
  <g transform="translate(32 37) scale(0.5)">
    <path d="${markPath}" fill="${lightFg}"/>
  </g>
  <text x="170" y="112"
    fill="${lightFg}"
    font-family="Inter, Montserrat, Avenir Next, Segoe UI, Arial, sans-serif"
    font-size="78"
    font-weight="700"
    letter-spacing="0.12em">EDGERUN</text>
</svg>
`

const logoHorizontalDarkSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 720 170" fill="none">
  <rect width="720" height="170" fill="${darkBg}"/>
  <g transform="translate(32 37) scale(0.5)">
    <path d="${markPath}" fill="${darkFg}"/>
  </g>
  <text x="170" y="112"
    fill="${darkFg}"
    font-family="Inter, Montserrat, Avenir Next, Segoe UI, Arial, sans-serif"
    font-size="78"
    font-weight="700"
    letter-spacing="0.12em">EDGERUN</text>
</svg>
`

const wordmarkSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 560 120" fill="none">
  <rect width="560" height="120" fill="${lightBg}"/>
  <text x="12" y="84"
    fill="${lightFg}"
    font-family="Inter, Montserrat, Avenir Next, Segoe UI, Arial, sans-serif"
    font-size="78"
    font-weight="700"
    letter-spacing="0.12em">EDGERUN</text>
</svg>
`

const wordmarkDarkSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 560 120" fill="none">
  <rect width="560" height="120" fill="${darkBg}"/>
  <text x="12" y="84"
    fill="${darkFg}"
    font-family="Inter, Montserrat, Avenir Next, Segoe UI, Arial, sans-serif"
    font-size="78"
    font-weight="700"
    letter-spacing="0.12em">EDGERUN</text>
</svg>
`

function transparentMarkSvg({ gradA = brandPrimary, gradB = '#1d4ed8' } = {}) {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" fill="none">
  <defs>
    <linearGradient id="edgerun-transparent-gradient" x1="8" y1="6" x2="56" y2="58" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="${gradA}" />
      <stop offset="1" stop-color="${gradB}" />
    </linearGradient>
  </defs>
  <path d="${markPath}" transform="translate(0 0) scale(0.3333333333)" fill="url(#edgerun-transparent-gradient)" />
</svg>
`
}

const iconAdaptiveSvg = transparentMarkSvg()

writeFileSync(path.join(brandDir, 'edgerun-mark.svg'), markSvg(lightFg))
writeFileSync(path.join(brandDir, 'edgerun-mark-light.svg'), markSvg(lightFg, lightBg))
writeFileSync(path.join(brandDir, 'edgerun-mark-dark.svg'), markSvg(darkFg, darkBg))
writeFileSync(path.join(brandDir, 'edgerun-wordmark.svg'), wordmarkSvg)
writeFileSync(path.join(brandDir, 'edgerun-wordmark-dark.svg'), wordmarkDarkSvg)
writeFileSync(path.join(brandDir, 'edgerun-logo.svg'), logoHorizontalSvg)
writeFileSync(path.join(brandDir, 'edgerun-logo-dark.svg'), logoHorizontalDarkSvg)
writeFileSync(path.join(publicDir, 'icon.svg'), iconAdaptiveSvg)
writeFileSync(path.join(publicDir, 'placeholder-logo.svg'), logoHorizontalSvg)
writeFileSync(path.join(publicDir, 'placeholder.svg'), markSvg(neutralMarkFg))

const iconLightTemp = path.join(brandDir, 'icon-light-temp.svg')
const iconDarkTemp = path.join(brandDir, 'icon-dark-temp.svg')
writeFileSync(iconLightTemp, transparentMarkSvg({ gradA: brandPrimary, gradB: '#2563eb' }))
writeFileSync(iconDarkTemp, transparentMarkSvg({ gradA: brandPrimary, gradB: '#1d4ed8' }))

function magick(args) {
  execFileSync('magick', args, { stdio: 'inherit' })
}

function pngResize(input, size, output) {
  magick([
    input,
    '-strip',
    '-filter',
    'Lanczos',
    '-resize',
    size,
    '-define',
    'png:compression-level=9',
    '-define',
    'png:compression-filter=5',
    '-define',
    'png:compression-strategy=1',
    output
  ])
}

function hasMagick() {
  const probe = spawnSync('magick', ['-version'], { stdio: 'ignore' })
  return probe.status === 0
}

if (hasMagick()) {
  pngResize(iconLightTemp, '32x32', path.join(publicDir, 'icon-light-32x32.png'))
  pngResize(iconDarkTemp, '32x32', path.join(publicDir, 'icon-dark-32x32.png'))
  pngResize(iconLightTemp, '180x180', path.join(publicDir, 'apple-icon.png'))
  pngResize(iconLightTemp, '192x192', path.join(publicDir, 'icon-192.png'))
  pngResize(iconDarkTemp, '512x512', path.join(publicDir, 'icon-512.png'))

  magick([
    iconLightTemp,
    '-strip',
    '-define',
    'icon:auto-resize=16,24,32,48',
    path.join(publicDir, 'favicon.ico')
  ])

  pngResize(path.join(brandDir, 'edgerun-logo.svg'), '256x144', path.join(publicDir, 'placeholder-logo.png'))
} else {
  console.warn(
    '[brand:generate] ImageMagick (magick) not found; keeping existing raster assets in public/.'
  )
}
rmSync(iconLightTemp, { force: true })
rmSync(iconDarkTemp, { force: true })

console.log('Generated brand assets in public/ and public/brand/')
