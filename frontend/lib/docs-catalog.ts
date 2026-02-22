import path from 'node:path'
import { existsSync, readdirSync } from 'node:fs'

export type DocsSource = {
  sourcePath: string
  slug?: string
  title?: string
  sourceLabel?: string
}

export type GeneratedApiSpec = {
  slug: string
  title: string
  description: string
  sourcePath: string
  mode: 'rust' | 'cli'
}

export function getDocsSources(repoRoot: string): DocsSource[] {
  const resolveExistingPath = (candidates: string[]): string => {
    for (const candidate of candidates) {
      if (existsSync(path.join(repoRoot, candidate))) return candidate
    }
    return candidates[0] || ''
  }
  const docsDir = path.join(repoRoot, 'docs')
  const docsEntries = existsSync(docsDir)
    ? readdirSync(docsDir)
        .filter((name) => name.endsWith('.mdx'))
        .map((name) => ({
          sourcePath: path.join('docs', name),
          ...(name === 'ONBOARDING.mdx' ? { slug: 'address-generation-workflow', title: 'Address Generation Workflow' } : {}),
          ...(name === 'ROUTED_TERMINAL_PROTOCOL_V2.mdx' ? { slug: 'routed-terminal-protocol-v2', title: 'Routed Terminal Protocol v2' } : {})
        }))
    : []

  return [
    { sourcePath: 'Whitepaper.mdx' },
    { sourcePath: 'Whitepaper-phase-2.mdx' },
    {
      sourcePath: resolveExistingPath([
        path.join('edgerun-apps', 'solana-vanity-address-generator', 'edgerun-vanity-client', 'README.mdx'),
        path.join('crates', 'edgerun-vanity-client', 'README.mdx')
      ]),
      slug: 'address-generator-cli',
      title: 'Address Generator CLI'
    },
    {
      sourcePath: resolveExistingPath([
        path.join('edgerun-apps', 'solana-vanity-address-generator', 'edgerun-vanity-payload', 'README.mdx'),
        path.join('crates', 'edgerun-vanity-payload', 'README.mdx')
      ]),
      slug: 'address-generator-payload',
      title: 'Address Generator Payload'
    },
    ...docsEntries
  ]
}

export const generatedApiSpecs: GeneratedApiSpec[] = [
  {
    slug: 'api-runtime-rust',
    title: 'Runtime Rust API',
    description: 'Public API surface for edgerun-runtime.',
    sourcePath: 'crates/edgerun-runtime/src/lib.rs',
    mode: 'rust'
  },
  {
    slug: 'api-types-rust',
    title: 'Types Rust API',
    description: 'Public API surface for edgerun-types.',
    sourcePath: 'crates/edgerun-types/src/lib.rs',
    mode: 'rust'
  },
  {
    slug: 'api-address-generator-payload-rust',
    title: 'Address Generator Payload Rust API',
    description: 'Public API surface for address generator payload crate.',
    sourcePath: 'edgerun-apps/solana-vanity-address-generator/edgerun-vanity-payload/src/lib.rs',
    mode: 'rust'
  },
  {
    slug: 'api-address-generator-cli',
    title: 'Address Generator CLI Reference',
    description: 'CLI argument surface for address generator client.',
    sourcePath: 'edgerun-apps/solana-vanity-address-generator/edgerun-vanity-client/src/main.rs',
    mode: 'cli'
  },
  {
    slug: 'api-edgerun-cli',
    title: 'Edgerun CLI Reference',
    description: 'CLI argument and command surface for edgerun-cli.',
    sourcePath: 'crates/edgerun-cli/src/main.rs',
    mode: 'cli'
  }
]
