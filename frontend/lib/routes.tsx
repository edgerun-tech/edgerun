// SPDX-License-Identifier: Apache-2.0
import type { Component } from 'solid-js'

import HomePage from '../app/page'
import DocsPage from '../app/docs/page'
import QuickStartPage from '../app/docs/getting-started/quick-start/page'
import TokenPage from '../app/token/page'
import RunPage from '../app/run/page'
import WorkersPage from '../app/workers/page'
import DashboardPage from '../app/dashboard/page'
import BlogPage from '../app/blog/page'
import BlogPostPage from '../app/blog/[slug]/page'
import JobDetailsPage from '../app/job/[id]/page'
import PrivacyPage from '../app/legal/privacy/page'
import TermsPage from '../app/legal/terms/page'
import SlaPage from '../app/legal/sla/page'
import StyleGuidePage from '../app/style-guide/page'
import VisionPage from '../app/vision/page'
import { blogPosts, jobs } from './content'
import { normalizeRoutePath } from './route-path'

export type SiteRoute = {
  path: string
  outputPath: string
  pageTitle: string
  chromeTitle: string
  description: string
  component: Component
}

const staticRoutes: SiteRoute[] = [
  {
    path: '/',
    outputPath: 'index.html',
    pageTitle: 'Edgerun',
    chromeTitle: 'Home',
    description: 'Dependable compute, financially enforced.',
    component: HomePage
  },
  {
    path: '/run/',
    outputPath: 'run/index.html',
    pageTitle: 'Execute Job',
    chromeTitle: 'Run Job',
    description: 'Submit jobs to Edgerun workers.',
    component: RunPage
  },
  {
    path: '/workers/',
    outputPath: 'workers/index.html',
    pageTitle: 'Workers',
    chromeTitle: 'Workers',
    description: 'Worker operations and status.',
    component: WorkersPage
  },
  {
    path: '/token/',
    outputPath: 'token/index.html',
    pageTitle: 'SOL Economics',
    chromeTitle: 'Token',
    description: 'SOL-based economics and settlement.',
    component: TokenPage
  },
  {
    path: '/dashboard/',
    outputPath: 'dashboard/index.html',
    pageTitle: 'Dashboard',
    chromeTitle: 'Dashboard',
    description: 'Operational views and on-chain truth.',
    component: DashboardPage
  },
  {
    path: '/docs/',
    outputPath: 'docs/index.html',
    pageTitle: 'Documentation',
    chromeTitle: 'Docs',
    description: 'Browse implementation docs.',
    component: DocsPage
  },
  {
    path: '/docs/getting-started/quick-start/',
    outputPath: 'docs/getting-started/quick-start/index.html',
    pageTitle: 'Quick Start',
    chromeTitle: 'Quick Start',
    description: 'Run bulk Solana address generation workflow end-to-end.',
    component: QuickStartPage
  },
  {
    path: '/blog/',
    outputPath: 'blog/index.html',
    pageTitle: 'Blog',
    chromeTitle: 'Blog',
    description: 'Protocol and release updates.',
    component: BlogPage
  },
  {
    path: '/style-guide/',
    outputPath: 'style-guide/index.html',
    pageTitle: 'Style Guide',
    chromeTitle: 'Style Guide',
    description: 'Design tokens and usage guide.',
    component: StyleGuidePage
  },
  {
    path: '/vision/',
    outputPath: 'vision/index.html',
    pageTitle: 'CloudOS Direction',
    chromeTitle: 'Direction',
    description: 'CloudOS product direction preserved in canonical frontend.',
    component: VisionPage
  },
  {
    path: '/legal/privacy/',
    outputPath: 'legal/privacy/index.html',
    pageTitle: 'Privacy Policy',
    chromeTitle: 'Privacy',
    description: 'Privacy terms.',
    component: PrivacyPage
  },
  {
    path: '/legal/terms/',
    outputPath: 'legal/terms/index.html',
    pageTitle: 'Terms of Service',
    chromeTitle: 'Terms',
    description: 'Terms for Edgerun services.',
    component: TermsPage
  },
  {
    path: '/legal/sla/',
    outputPath: 'legal/sla/index.html',
    pageTitle: 'Service Level Agreement',
    chromeTitle: 'SLA',
    description: 'Service-level terms.',
    component: SlaPage
  }
]

const blogRoutes: SiteRoute[] = blogPosts.map((post) => ({
  path: `/blog/${post.slug}/`,
  outputPath: `blog/${post.slug}/index.html`,
  pageTitle: post.title,
  chromeTitle: post.title,
  description: post.excerpt,
  component: () => <BlogPostPage slug={post.slug} />
}))

const jobRoutes: SiteRoute[] = jobs.map((job) => ({
  path: `/job/${job.id}/`,
  outputPath: `job/${job.id}/index.html`,
  pageTitle: `Job ${job.id}`,
  chromeTitle: `Job ${job.id}`,
  description: 'Job execution details and timeline.',
  component: () => <JobDetailsPage id={job.id} />
}))

const allRoutes: SiteRoute[] = [...staticRoutes, ...blogRoutes, ...jobRoutes]

export function getAllSiteRoutes(): SiteRoute[] {
  return allRoutes
}

export function getSiteRouteMap(): Record<string, Component> {
  const map: Record<string, Component> = {}
  for (const route of allRoutes) map[route.path] = route.component
  return map
}

export function getRouteChromeTitle(pathname: string): string {
  const route = normalizeRoutePath(pathname)
  const found = allRoutes.find((entry) => entry.path === route)
  return found ? found.chromeTitle : 'Edgerun'
}
