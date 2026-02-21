import { For } from 'solid-js'
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'
import { Badge } from '../ui/badge'

const useCases = [
  { title: 'Machine Learning Inference', description: 'Run ML models with verifiable outputs.', tags: ['AI/ML', 'Inference'] },
  { title: 'Data Processing Pipelines', description: 'Process datasets with cryptographic guarantees.', tags: ['Data', 'ETL'] },
  { title: 'Smart Contract Oracles', description: 'Provide off-chain computation results on-chain.', tags: ['Web3', 'Oracles'] },
  { title: 'Image & Video Processing', description: 'Transform media with verifiable outputs.', tags: ['Media', 'Processing'] },
  { title: 'Cryptographic Operations', description: 'Execute heavy cryptographic workloads.', tags: ['Crypto', 'Security'] },
  { title: 'Scientific Computing', description: 'Run deterministic simulation workloads.', tags: ['Science', 'Research'] }
]

export function UseCases() {
  return (
    <section class="bg-card/50 py-20">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto mb-16 max-w-3xl text-center">
          <h2 class="mb-4 text-balance text-3xl font-bold md:text-4xl">Built for Diverse Workloads</h2>
          <p class="text-lg text-muted-foreground">From AI inference to data processing, Edgerun handles WASM-compatible workloads.</p>
        </div>
        <div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          <For each={useCases}>{(useCase: (typeof useCases)[number]) => (
            <Card class="transition-colors hover:border-accent/50">
              <CardHeader>
                <div class="mb-3 flex flex-wrap gap-2">
                  <For each={useCase.tags}>{(tag: string) => <Badge>{tag}</Badge>}</For>
                </div>
                <CardTitle class="text-xl">{useCase.title}</CardTitle>
              </CardHeader>
              <CardContent>
                <p class="text-sm leading-relaxed text-muted-foreground">{useCase.description}</p>
              </CardContent>
            </Card>
          )}</For>
        </div>
      </div>
    </section>
  )
}
