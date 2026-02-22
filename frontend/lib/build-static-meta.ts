import { readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'

export type BuildMetaConfig = {
  projectRoot: string
  distRoot: string
  siteUrl: string
  siteDomain: string
  currentVersion: string
  buildNumber: string
  solanaCluster: string
  solanaRpcUrl: string
  treasuryAccount: string
}

export function writeStaticSiteMetadata(
  config: BuildMetaConfig,
  paths: string[],
  schedulerWikiMdx: string
): void {
  const {
    projectRoot,
    distRoot,
    siteUrl,
    siteDomain,
    currentVersion,
    buildNumber,
    solanaCluster,
    solanaRpcUrl,
    treasuryAccount
  } = config

  const themeConfig = JSON.parse(readFileSync(path.join(projectRoot, 'config', 'brand-theme.json'), 'utf8')) as {
    name: string
    shortName: string
    description: string
    colors: { darkBackground: string; brandPrimary: string }
  }

  writeFileSync(path.join(distRoot, 'robots.txt'), `User-agent: *\nAllow: /\nSitemap: ${siteUrl}/sitemap.xml\n`, 'utf8')
  writeFileSync(
    path.join(distRoot, 'sitemap.xml'),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${paths.map((href) => `  <url><loc>${siteUrl}${href}</loc></url>`).join('\n')}\n</urlset>\n`,
    'utf8'
  )

  writeFileSync(
    path.join(distRoot, 'manifest.webmanifest'),
    JSON.stringify(
      {
        name: themeConfig.name,
        short_name: themeConfig.shortName,
        description: themeConfig.description,
        start_url: '/',
        display: 'standalone',
        background_color: themeConfig.colors.darkBackground,
        theme_color: themeConfig.colors.brandPrimary,
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png' },
          { src: '/icon.svg', sizes: 'any', type: 'image/svg+xml' }
        ]
      },
      null,
      2
    ) + '\n',
    'utf8'
  )

  writeFileSync(path.join(distRoot, 'build-meta.json'), JSON.stringify({ version: currentVersion, buildNumber, siteUrl, solanaCluster, solanaRpcUrl, treasuryAccount }, null, 2) + '\n', 'utf8')
  if (siteDomain) writeFileSync(path.join(distRoot, 'CNAME'), `${siteDomain}\n`, 'utf8')

  const llmsBase = [
    '# Edgerun',
    '',
    '> Deterministic WASM compute with SOL settlement.',
    '',
    `Version: ${currentVersion}`,
    `Build: ${buildNumber}`,
    `Docs: ${siteUrl}/docs/${currentVersion}/`,
    `Releases: ${siteUrl}/releases/`
  ].join('\n')
  writeFileSync(path.join(distRoot, 'llms.txt'), `${llmsBase}\n`, 'utf8')
  writeFileSync(path.join(distRoot, 'llms-full.txt'), `${llmsBase}\n\n${schedulerWikiMdx}\n`, 'utf8')
}
