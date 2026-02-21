export function cx(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(' ')
}

export const uiTheme = {
  focusRing: 'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
  surfaces: {
    card: 'rounded-lg border border-border bg-card',
    panel: 'rounded-lg border border-border bg-background/60'
  },
  text: {
    body: 'text-foreground',
    muted: 'text-muted-foreground'
  },
  button: {
    base: 'inline-flex items-center justify-center rounded-md font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed',
    variant: {
      default: 'bg-primary text-primary-foreground hover:bg-primary/90',
      secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
      outline: 'border border-border bg-transparent text-foreground hover:bg-muted/60',
      ghost: 'text-foreground hover:bg-muted/50',
      destructive: 'bg-destructive text-destructive-foreground hover:bg-destructive/90'
    },
    size: {
      sm: 'h-8 px-3 text-sm',
      md: 'h-10 px-4 text-sm',
      lg: 'h-11 px-5 text-base'
    }
  },
  badge: {
    base: 'inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide',
    variant: {
      default: 'border-border bg-muted/50 text-foreground',
      secondary: 'border-border bg-secondary text-secondary-foreground',
      outline: 'border-border bg-transparent text-foreground',
      destructive: 'border-destructive/50 bg-destructive/20 text-destructive-foreground'
    }
  },
  input: {
    base: 'h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none ring-offset-background placeholder:text-muted-foreground',
    active: 'focus-visible:ring-2 focus-visible:ring-ring'
  },
  textarea: {
    base: 'min-h-24 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none ring-offset-background placeholder:text-muted-foreground',
    active: 'focus-visible:ring-2 focus-visible:ring-ring'
  },
  label: {
    base: 'text-sm font-medium leading-none'
  },
  separator: {
    horizontal: 'h-px w-full bg-border',
    vertical: 'h-full w-px bg-border'
  },
  alert: {
    base: 'relative w-full rounded-lg border p-4',
    variant: {
      default: 'border-border bg-card text-foreground',
      destructive: 'border-destructive/50 bg-destructive/10 text-destructive-foreground'
    }
  },
  table: {
    container: 'relative w-full overflow-x-auto',
    table: 'w-full caption-bottom text-sm',
    header: '[&_tr]:border-b',
    body: '[&_tr:last-child]:border-0',
    footer: 'border-t bg-muted/50 font-medium [&>tr]:last:border-b-0',
    row: 'border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted',
    head: 'h-10 px-3 text-left align-middle font-medium text-foreground',
    cell: 'p-3 align-middle',
    caption: 'mt-3 text-sm text-muted-foreground'
  }
} as const
