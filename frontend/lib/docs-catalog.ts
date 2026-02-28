// SPDX-License-Identifier: Apache-2.0
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
  }
]
