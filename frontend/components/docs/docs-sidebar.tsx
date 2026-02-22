import { For } from 'solid-js'
import { getDocsNav } from '../../lib/docs-nav'

type DocsSidebarProps = {
  version?: string
  showSearch?: boolean
  class?: string
}

export function DocsSidebar(props: DocsSidebarProps) {
  const version = () => props.version || 'main'
  const searchInputId = () => `docs-search-input-${version().replace(/[^a-z0-9-]/gi, '-')}`
  const navItems = () => getDocsNav(version())
  const serializedSearchIndex = () => JSON.stringify(
    navItems().map((item) => ({
      title: item.label,
      href: item.href,
      text: item.label
    }))
  )
  return (
    <aside class={props.class || 'h-fit rounded-lg border border-border bg-card/50 p-4'}>
      <p class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Documentation</p>
      {props.showSearch && (
        <div
          class="mb-3 space-y-2"
          data-docs-search
          data-docs-version={version()}
          data-docs-search-index={serializedSearchIndex()}
          role="search"
          aria-busy="true"
        >
          <p class="text-xs font-medium text-muted-foreground">Search docs</p>
          <input
            id={searchInputId()}
            aria-label="Search docs"
            data-docs-search-input
            type="search"
            placeholder="Search by keyword..."
            disabled
            class="h-9 w-full rounded-md border border-input bg-background px-3 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-70"
          />
          <div data-docs-search-results aria-live="polite" class="min-h-10 max-h-48 overflow-auto space-y-1 text-xs">
            <p class="rounded border border-border bg-muted/20 px-2 py-1 text-muted-foreground">Type at least 2 characters.</p>
          </div>
        </div>
      )}
      <nav class="space-y-1 text-sm">
        <For each={navItems()}>{(item) => (
          <a href={item.href} data-nav-link class="block rounded px-2 py-1 hover:bg-muted/50">
            {item.label}
          </a>
        )}</For>
      </nav>
    </aside>
  )
}
