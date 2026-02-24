// SPDX-License-Identifier: Apache-2.0
import { For } from 'solid-js'
import { PageHero } from './page-hero'
import { PageShell } from './page-shell'
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'

export type LegalSection = {
  title: string
  paragraphs: string[]
}

type LegalDocumentPageProps = {
  title: string
  description: string
  effectiveDate: string
  sections: LegalSection[]
}

export function LegalDocumentPage(props: LegalDocumentPageProps) {
  return (
    <PageShell>
      <PageHero title={props.title} badge={`Effective ${props.effectiveDate}`} badgeVariant="outline" description={props.description} maxWidthClass="max-w-5xl" />
      <section class="mx-auto max-w-5xl space-y-6 px-4 py-8 sm:px-6 lg:px-8">
        <For each={props.sections}>{(section) => (
          <Card>
            <CardHeader><CardTitle>{section.title}</CardTitle></CardHeader>
            <CardContent class="space-y-4">
              <For each={section.paragraphs}>{(paragraph) => (
                <p class="leading-relaxed text-muted-foreground">{paragraph}</p>
              )}</For>
            </CardContent>
          </Card>
        )}</For>
      </section>
    </PageShell>
  )
}
