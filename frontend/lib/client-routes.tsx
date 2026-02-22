import type { Component } from 'solid-js'

import { blogPosts, jobs } from './content'

type LoadedComponent = { default: Component }
type RouteLoader = () => Promise<LoadedComponent>

type ClientRouteDef = {
  chromeTitle: string
  load: RouteLoader
}

type ClientRouteMap = Record<string, ClientRouteDef>

const staticRoutes: ClientRouteMap = {
  '/': {
    chromeTitle: 'Home',
    load: () => import('../app/page')
  },
  '/run/': {
    chromeTitle: 'Run Job',
    load: () => import('../app/run/page')
  },
  '/workers/': {
    chromeTitle: 'Workers',
    load: () => import('../app/workers/page')
  },
  '/token/': {
    chromeTitle: 'Token',
    load: () => import('../app/token/page')
  },
  '/dashboard/': {
    chromeTitle: 'Dashboard',
    load: () => import('../app/dashboard/page')
  },
  '/docs/': {
    chromeTitle: 'Docs',
    load: () => import('../app/docs/page')
  },
  '/docs/getting-started/quick-start/': {
    chromeTitle: 'Quick Start',
    load: () => import('../app/docs/getting-started/quick-start/page')
  },
  '/blog/': {
    chromeTitle: 'Blog',
    load: () => import('../app/blog/page')
  },
  '/style-guide/': {
    chromeTitle: 'Style Guide',
    load: () => import('../app/style-guide/page')
  },
  '/legal/privacy/': {
    chromeTitle: 'Privacy',
    load: () => import('../app/legal/privacy/page')
  },
  '/legal/terms/': {
    chromeTitle: 'Terms',
    load: () => import('../app/legal/terms/page')
  },
  '/legal/sla/': {
    chromeTitle: 'SLA',
    load: () => import('../app/legal/sla/page')
  }
}

const blogRoutes: ClientRouteMap = Object.fromEntries(
  blogPosts.map((post) => [
    `/blog/${post.slug}/`,
    {
      chromeTitle: post.title,
      load: async () => {
        const mod = await import('../app/blog/[slug]/page')
        const Page = mod.default
        return {
          default: (() => <Page slug={post.slug} />) as Component
        }
      }
    }
  ])
)

const jobRoutes: ClientRouteMap = Object.fromEntries(
  jobs.map((job) => [
    `/job/${job.id}/`,
    {
      chromeTitle: `Job ${job.id}`,
      load: async () => {
        const mod = await import('../app/job/[id]/page')
        const Page = mod.default
        return {
          default: (() => <Page id={job.id} />) as Component
        }
      }
    }
  ])
)

const routeMap: ClientRouteMap = {
  ...staticRoutes,
  ...blogRoutes,
  ...jobRoutes
}

export function normalizeClientRoutePath(pathname: string): string {
  const cleaned = pathname.replace(/index\.html$/, '')
  if (!cleaned) return '/'
  return cleaned.endsWith('/') ? cleaned : `${cleaned}/`
}

export function getClientRouteChromeTitle(pathname: string): string {
  const route = normalizeClientRoutePath(pathname)
  return routeMap[route]?.chromeTitle || 'Edgerun'
}

export function hasClientRoute(pathname: string): boolean {
  const route = normalizeClientRoutePath(pathname)
  return route in routeMap
}

export async function loadClientRouteComponent(pathname: string): Promise<Component | null> {
  const route = normalizeClientRoutePath(pathname)
  const entry = routeMap[route]
  if (!entry) return null
  const loaded = await entry.load()
  return loaded.default
}
