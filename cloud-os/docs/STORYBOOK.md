# Storybook for CloudOS

Interactive component development environment for CloudOS morphable result views.

## Getting Started

```bash
# Start Storybook dev server
npm run storybook

# Build static Storybook
npm run build-storybook
```

Storybook will be available at **http://localhost:6006**

## Components

### Result Views

All 10 morphable result view components have stories:

| Component | Story | Description |
|-----------|-------|-------------|
| **PreviewCard** | `Results/PreviewCard` | Default view for any result |
| **Timeline** | `Results/Timeline` | Event sequences, deployment history |
| **LogViewer** | `Results/LogViewer` | Terminal-style log output |
| **DataTable** | `Results/DataTable` | Sortable, filterable tables |
| **JSONTree** | `Results/JSONTree` | Expandable JSON tree |
| **FileGrid** | `Results/FileGrid` | File/folder grid with thumbnails |
| **CodeDiffViewer** | `Results/CodeDiffViewer` | Git diff viewer (coming soon) |
| **EmailReader** | `Results/EmailReader` | Email conversation view (coming soon) |
| **DocViewer** | `Results/DocViewer` | Markdown renderer (coming soon) |
| **MediaGallery** | `Results/MediaGallery` | Image/video gallery (coming soon) |

## Example Stories

### PreviewCard
```typescript
// Default with actions
<PreviewCard 
  response={{
    success: true,
    data: { message: 'Success' },
    ui: {
      viewType: 'preview',
      title: 'Window Opened',
      actions: [{ label: 'Close', intent: 'close window' }]
    }
  }}
/>
```

### Timeline
```typescript
// Deployment history
<Timeline 
  response={{
    success: true,
    data: [
      { 
        timestamp: '2024-02-17T13:00:00Z',
        title: 'Production Deploy',
        type: 'deployment'
      }
    ],
    ui: { viewType: 'timeline', title: 'Deployments' }
  }}
/>
```

### LogViewer
```typescript
// Application logs
<LogViewer 
  response={{
    success: true,
    data: [
      { level: 'error', message: 'Connection failed' }
    ],
    ui: { viewType: 'log-viewer', title: 'Errors' }
  }}
/>
```

## Addons

- **@storybook/addon-docs** - Documentation generation
- **@storybook/addon-a11y** - Accessibility testing
- **@storybook/addon-vitest** - Visual regression testing
- **@chromatic-com/storybook** - Chromatic integration

## Configuration

### main.ts
```typescript
{
  "stories": [
    "../src/**/*.mdx",
    "../src/**/*.stories.@(js|jsx|mjs|ts|tsx)"
  ],
  "addons": [
    "@chromatic-com/storybook",
    "@storybook/addon-vitest",
    "@storybook/addon-a11y",
    "@storybook/addon-docs"
  ],
  "framework": "storybook-solidjs-vite"
}
```

## Creating New Stories

1. Create `ComponentName.stories.tsx` next to your component
2. Define meta with component reference
3. Add stories with different args
4. Run `npm run storybook` to see changes

### Example
```typescript
import { Meta, StoryObj } from 'storybook-solidjs-vite';
import { MyComponent } from './MyComponent';

const meta = {
  title: 'Results/MyComponent',
  component: MyComponent,
} satisfies Meta<typeof MyComponent>;

export default meta;

export const Default: Story = {
  args: {
    // Your component props here
  }
};
```

## Testing

```bash
# Run visual tests
npm run test:storybook

# Build and preview
npm run build-storybook
npx serve storybook-static
```

## CI/CD

Storybook is automatically built and can be deployed to:
- Chromatic for visual testing
- Any static hosting (Cloudflare Pages, Vercel, etc.)

```bash
# Build for deployment
npm run build-storybook

# Deploy storybook-static folder
```
