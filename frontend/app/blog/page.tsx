// SPDX-License-Identifier: Apache-2.0
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/card'
import { PageHero } from '../../components/layout/page-hero'
import { PageShell } from '../../components/layout/page-shell'
import { blogPosts, formatShortDate } from '../../lib/content'

export default function BlogPage() {
  const featured = blogPosts[0]

  return (
    <PageShell>
      <PageHero
        title="Blog"
        badge="Protocol Notes"
        badgeVariant="secondary"
        description="Release writeups and architecture notes aligned with shipped code and docs versions."
      />
      <section class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div class="grid gap-4 md:grid-cols-2">
          <a href={featured ? `/blog/${featured.slug}/` : '/blog/'}>
            <Card class="h-full transition-colors hover:border-primary/50">
              <CardHeader>
                <CardTitle>{featured?.title || 'Introducing Edgerun'}</CardTitle>
                <CardDescription>{featured?.excerpt || 'Deterministic compute with verifiable execution and transparent receipts.'}</CardDescription>
              </CardHeader>
              <CardContent><p class="text-sm text-muted-foreground">{featured ? `${formatShortDate(featured.publishedAt)} • ${featured.readingTime} min read` : 'Published soon'}</p></CardContent>
            </Card>
          </a>
          <Card>
            <CardHeader>
              <CardTitle>Start Here</CardTitle>
              <CardDescription>If you are new, read why Edgerun exists before diving into endpoints and runtime knobs.</CardDescription>
            </CardHeader>
            <CardContent>
              <a href={featured ? `/blog/${featured.slug}/` : '/blog/'} class="text-sm text-primary hover:underline">
                Read: Why Edgerun Exists
              </a>
            </CardContent>
          </Card>
        </div>
      </section>
    </PageShell>
  )
}
