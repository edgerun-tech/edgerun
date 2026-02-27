// SPDX-License-Identifier: Apache-2.0
import MarkdownIt from 'markdown-it'
import { evaluateSync } from '@mdx-js/mdx'

type MdxAstNode =
  | null
  | undefined
  | boolean
  | string
  | number
  | {
    type?: any
    props?: Record<string, unknown>
  }
  | MdxAstNode[]

type HighlighterLike = {
  codeToHtml: (code: string, options: { lang: string; theme: string }) => string
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

export function createDocsRenderer(shiki: HighlighterLike) {
  const markdown = new MarkdownIt({
    html: false,
    linkify: true,
    typographer: true,
    highlight: (code: string, lang: string) => {
      try {
        return shiki.codeToHtml(code, { lang: (lang || 'plaintext').toLowerCase(), theme: 'github-dark' })
      } catch {
        return `<pre class="shiki"><code>${escapeHtml(code)}</code></pre>`
      }
    }
  })

  const mdxFragment = Symbol.for('edgerun.mdx.fragment')

  function mdxJsx(type: any, props: Record<string, unknown> | null, key?: string | number): MdxAstNode {
    return { type, props: { ...(props || {}), key } }
  }

  function stripFrontmatter(content: string): string {
    return content.replace(/^---\n[\s\S]*?\n---\n?/m, '')
  }

  function normalizeDocsTerminology(content: string, sourcePath: string): string {
    if (!sourcePath) return content
    return content
  }

  function renderMdxAst(node: MdxAstNode): string {
    if (node === null || node === undefined || node === false || node === true) return ''
    if (Array.isArray(node)) return node.map(renderMdxAst).join('')
    if (typeof node === 'string' || typeof node === 'number') return escapeHtml(String(node))
    const type = node.type
    const props = node.props || {}

    if (type === mdxFragment) return renderMdxAst(props.children as MdxAstNode)
    if (typeof type === 'function') return renderMdxAst(type(props))
    if (typeof type !== 'string') return renderMdxAst(props.children as MdxAstNode)

    const voidTags = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'source'])
    const attrs = Object.entries(props)
      .filter(([name]) => name !== 'children' && name !== 'key')
      .map(([name, value]) => {
        if (value === null || value === undefined || value === false) return ''
        const attrName = name === 'className' ? 'class' : name
        if (value === true) return ` ${attrName}`
        if (typeof value === 'string' || typeof value === 'number') return ` ${attrName}="${escapeHtml(String(value))}"`
        return ''
      })
      .join('')

    if (voidTags.has(type)) return `<${type}${attrs} />`
    return `<${type}${attrs}>${renderMdxAst(props.children as MdxAstNode)}</${type}>`
  }

  function decodeHtmlEntities(value: string): string {
    return value
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>')
      .replaceAll('&quot;', '"')
      .replaceAll('&#39;', "'")
      .replaceAll('&amp;', '&')
  }

  function highlightCodeBlocksHtml(html: string): string {
    return html.replace(/<pre([^>]*)>\s*<code([^>]*)>([\s\S]*?)<\/code>\s*<\/pre>/g, (full, preAttrsRaw, codeAttrsRaw, codeRaw) => {
      const preAttrs = String(preAttrsRaw || '')
      if (/\bshiki\b/i.test(preAttrs)) return full
      const codeAttrs = String(codeAttrsRaw || '')
      const lang = (codeAttrs.match(/language-([a-z0-9_-]+)/i)?.[1] || 'plaintext').toLowerCase()
      const code = decodeHtmlEntities(String(codeRaw || ''))
      try {
        return shiki.codeToHtml(code, { lang, theme: 'github-dark' })
      } catch {
        return `<pre class="shiki"><code>${escapeHtml(code)}</code></pre>`
      }
    })
  }

  function renderMdxContent(content: string): string {
    const source = stripFrontmatter(content)
    const Callout = (data: Record<string, unknown>) => mdxJsx('aside', {
      className: 'rounded-lg border border-border bg-card/70 p-4',
      children: [
        data.title ? mdxJsx('p', { className: 'mb-1 text-sm font-semibold text-foreground', children: String(data.title) }) : null,
        mdxJsx('div', { className: 'text-sm text-muted-foreground', children: data.children })
      ]
    })
    const CodeBlock = (data: Record<string, unknown>) => {
      const code = typeof data.code === 'string' ? data.code : typeof data.children === 'string' ? data.children : ''
      const language = typeof data.language === 'string' ? data.language : 'plaintext'
      return mdxJsx('pre', {
        className: 'overflow-x-auto rounded-lg border border-border bg-black/40 p-4',
        children: mdxJsx('code', { className: `language-${language}`, children: code })
      })
    }

    const evaluated: any = evaluateSync(source, {
      Fragment: mdxFragment,
      jsx: mdxJsx,
      jsxs: mdxJsx,
      development: false
    })
    const tree = evaluated.default({ components: { Callout, CodeBlock } })
    return highlightCodeBlocksHtml(renderMdxAst(tree))
  }

  function renderDocsContent(content: string, sourcePath: string): string {
    const normalized = normalizeDocsTerminology(content, sourcePath)
    if (sourcePath.endsWith('.mdx')) {
      return renderMdxContent(normalized)
    }
    return markdown.render(normalized)
  }

  function stripMarkdownForSearch(content: string): string {
    return content
      .replace(/^---\n[\s\S]*?\n---\n?/m, '')
      .replace(/`{3}[\s\S]*?`{3}/g, ' ')
      .replace(/`[^`]+`/g, ' ')
      .replace(/!\[[^\]]*\]\([^)]+\)/g, ' ')
      .replace(/\[[^\]]+\]\([^)]+\)/g, ' ')
      .replace(/<[^>]+>/g, ' ')
      .replace(/[#>*_-]+/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
  }

  return {
    normalizeDocsTerminology,
    renderDocsContent,
    stripMarkdownForSearch,
    renderMarkdown: (content: string) => markdown.render(content)
  }
}
