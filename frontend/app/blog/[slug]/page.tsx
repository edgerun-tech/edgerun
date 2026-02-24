// SPDX-License-Identifier: Apache-2.0
import { For, untrack } from 'solid-js'
import { Badge } from '../../../components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '../../../components/ui/card'
import { PageHero } from '../../../components/layout/page-hero'
import { PageShell } from '../../../components/layout/page-shell'
import { Separator } from '../../../components/ui/separator'
import { blogPosts, formatShortDate } from '../../../lib/content'

type BlogPostPageProps = {
  slug?: string
}

export default function BlogPostPage(props: BlogPostPageProps) {
  const fallbackPost = {
    slug: 'post',
    title: 'Why Edgerun Exists',
    excerpt: 'Dependable compute needs visible economics and verifiable execution.',
    publishedAt: new Date().toISOString(),
    readingTime: 4,
    tags: ['Why Edgerun'],
    sections: [
      {
        heading: 'Purpose',
        paragraphs: [
          'Edgerun focuses on deterministic execution and settlement visibility so operators can trust production runs.'
        ]
      }
    ],
    author: { name: 'Edgerun', role: 'Team' }
  }
  const slug = untrack(() => props.slug)
  const post = blogPosts.find((item) => item.slug === slug) || blogPosts[0] || fallbackPost
  const related = blogPosts.filter((item) => item.slug !== post.slug).slice(0, 3)

  return (
    <PageShell>
      <PageHero
        title={post.title}
        badge="Blog"
        description={post.excerpt}
        actions={<a href="/blog/" class="text-sm text-primary hover:underline">Back to Blog</a>}
      />

      <section class="mx-auto max-w-4xl px-4 py-8 sm:px-6 lg:px-8" data-testid="blog-article">
        <div class="mb-6 flex flex-wrap gap-2">
          <For each={post.tags}>{(tag) => <Badge variant="secondary">{tag}</Badge>}</For>
        </div>
        <div class="mb-4 text-sm text-muted-foreground">
          <p>{post.author.name} • {post.author.role}</p>
          <p>{formatShortDate(post.publishedAt)} • {post.readingTime} min read</p>
        </div>
        <Separator class="my-6" />
        <article class="space-y-8 text-muted-foreground leading-relaxed">
          <For each={post.sections}>{(section) => (
            <section>
              <h2 class="mb-3 text-xl font-semibold text-foreground">{section.heading}</h2>
              <div class="space-y-3">
                <For each={section.paragraphs}>{(paragraph) => (
                  <p>{paragraph}</p>
                )}</For>
              </div>
            </section>
          )}</For>
        </article>
      </section>

      <section class="border-t border-border bg-card/30">
        <div class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <h2 class="mb-4 text-2xl font-bold">Related Posts</h2>
          <div class="grid gap-4 md:grid-cols-3">
            <For each={related}>{(item) => (
              <a href={`/blog/${item.slug}/`}>
                <Card class="h-full transition-colors hover:border-primary/50">
                  <CardHeader>
                    <CardTitle class="text-lg">{item.title}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p class="text-sm text-muted-foreground">{item.excerpt}</p>
                  </CardContent>
                </Card>
              </a>
            )}</For>
          </div>
        </div>
      </section>
    </PageShell>
  )
}
