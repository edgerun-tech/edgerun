// SPDX-License-Identifier: Apache-2.0
import { Input } from './ui/input'
import { Button } from './ui/button'

export function FooterLeadForm() {
  return (
    <div class="rounded-lg border border-border bg-background/60 p-4">
      <div class="flex flex-col gap-3 sm:flex-row sm:items-center">
        <div class="flex-1">
          <p class="text-sm font-semibold">Get release updates</p>
          <p class="text-xs text-muted-foreground">Product and protocol notes by email.</p>
        </div>
        <div class="flex w-full gap-2 sm:w-auto">
          <Input placeholder="you@company.com" type="email" />
          <Button variant="outline" disabled>
            <span data-generating-label>Generating</span>
          </Button>
        </div>
      </div>
    </div>
  )
}
