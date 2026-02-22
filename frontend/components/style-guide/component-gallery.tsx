// SPDX-License-Identifier: Apache-2.0
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Badge } from '../ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card'

export function ComponentGallery() {
  return (
    <div class="space-y-8">
      <div class="space-y-4">
        <h4 class="text-lg font-semibold">Buttons</h4>
        <div class="flex flex-wrap items-center gap-3">
          <Button>Primary Button</Button>
          <Button variant="secondary">Secondary</Button>
          <Button variant="outline">Outline</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="destructive">Destructive</Button>
          <Button size="sm">Small</Button>
          <Button size="lg">Large</Button>
        </div>
      </div>

      <div class="space-y-4">
        <h4 class="text-lg font-semibold">Badges</h4>
        <div class="flex flex-wrap items-center gap-2">
          <Badge>Default</Badge>
          <Badge variant="secondary">Secondary</Badge>
          <Badge variant="outline">Outline</Badge>
          <Badge variant="destructive">Destructive</Badge>
        </div>
      </div>

      <div class="space-y-4">
        <h4 class="text-lg font-semibold">Input Fields</h4>
        <div class="max-w-md space-y-3">
          <Input placeholder="Default input" />
          <Input placeholder="Disabled input" disabled />
        </div>
      </div>

      <div class="space-y-4">
        <h4 class="text-lg font-semibold">Cards</h4>
        <div class="grid max-w-3xl gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Card Title</CardTitle>
              <CardDescription>Card description with additional context about the content</CardDescription>
            </CardHeader>
            <CardContent>
              <p class="text-sm text-muted-foreground">Card content goes here. Cards are used to group related information.</p>
            </CardContent>
          </Card>
          <Card class="bg-muted">
            <CardHeader>
              <CardTitle>Muted Card</CardTitle>
              <CardDescription>Cards can have different background colors</CardDescription>
            </CardHeader>
            <CardContent>
              <p class="text-sm text-muted-foreground">Use muted backgrounds for less-prominent content.</p>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
