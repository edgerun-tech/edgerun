// SPDX-License-Identifier: Apache-2.0
import { PageHero } from './page-hero'
import { PageShell } from './page-shell'
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'
import { GeneratingIndicator } from '../ui/generating-indicator'

type LegalPlaceholderPageProps = {
  title: string
  description: string
  cardTitle: string
  body: string
}

export function LegalPlaceholderPage(props: LegalPlaceholderPageProps) {
  return (
    <PageShell>
      <PageHero title={props.title} badge="Generating" badgeVariant="outline" description={props.description} maxWidthClass="max-w-5xl" />
      <section class="mx-auto max-w-5xl px-4 py-8 sm:px-6 lg:px-8">
        <Card>
          <CardHeader><CardTitle>{props.cardTitle}</CardTitle></CardHeader>
          <CardContent>
            <p class="text-muted-foreground">{props.body}</p>
            <GeneratingIndicator class="mt-3 inline-flex text-sm" />
          </CardContent>
        </Card>
      </section>
    </PageShell>
  )
}
