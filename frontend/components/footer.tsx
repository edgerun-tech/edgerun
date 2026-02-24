// SPDX-License-Identifier: Apache-2.0
import { For } from 'solid-js'
import { FooterLeadForm } from './footer-lead-form'
import { siteLinks } from '../lib/site-links'
import { docsApiReferenceHref, docsQuickStartHref } from '../lib/docs-links'

const footerLinks = {
  product: [
    { href: '/run/', label: 'Run Job' },
    { href: '/workers/', label: 'Workers' },
    { href: '/dashboard/', label: 'Dashboard' },
    { href: '/docs/', label: 'Documentation' }
  ],
  resources: [
    { href: docsQuickStartHref(), label: 'Getting Started' },
    { href: docsApiReferenceHref('main'), label: 'API Reference' },
    { href: '/blog/', label: 'Blog' },
    { href: '/style-guide/', label: 'Style Guide' }
  ],
  legal: [
    { href: '/legal/privacy/', label: 'Privacy Policy' },
    { href: '/legal/terms/', label: 'Terms of Service' },
    { href: '/legal/sla/', label: 'Service Level Agreement' }
  ],
  community: [
    { href: siteLinks.community.github, label: 'GitHub' },
    { href: siteLinks.community.discord, label: 'Discord' }
  ]
}

export function Footer() {
  return (
    <footer class="mt-auto border-t border-border bg-card">
      <div class="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
        <div class="mb-8">
          <FooterLeadForm />
        </div>

        <div class="grid grid-cols-2 gap-8 md:grid-cols-4">
          <div>
            <h3 class="mb-4 font-semibold text-foreground">Product</h3>
            <ul class="space-y-3">
              <For each={footerLinks.product}>{(link: (typeof footerLinks.product)[number]) => <li><a href={link.href} class="text-sm text-muted-foreground hover:text-foreground">{link.label}</a></li>}</For>
            </ul>
          </div>
          <div>
            <h3 class="mb-4 font-semibold text-foreground">Resources</h3>
            <ul class="space-y-3">
              <For each={footerLinks.resources}>{(link: (typeof footerLinks.resources)[number]) => <li><a href={link.href} class="text-sm text-muted-foreground hover:text-foreground">{link.label}</a></li>}</For>
            </ul>
          </div>
          <div>
            <h3 class="mb-4 font-semibold text-foreground">Legal</h3>
            <ul class="space-y-3">
              <For each={footerLinks.legal}>{(link: (typeof footerLinks.legal)[number]) => <li><a href={link.href} class="text-sm text-muted-foreground hover:text-foreground">{link.label}</a></li>}</For>
            </ul>
          </div>
          <div>
            <h3 class="mb-4 font-semibold text-foreground">Community</h3>
            <ul class="space-y-3">
              <For each={footerLinks.community}>{(link: (typeof footerLinks.community)[number]) => (
                <li>
                  <a href={link.href} target="_blank" rel="noreferrer" class="text-sm text-muted-foreground hover:text-foreground">
                    {link.label}
                  </a>
                </li>
              )}</For>
            </ul>
          </div>
        </div>

        <div class="mt-10 border-t border-border pt-6 flex items-center justify-between gap-4">
          <div class="flex items-center gap-2">
            <img src="/brand/edgerun-mark.svg" alt="Edgerun mark" width="24" height="24" />
            <span class="font-bold">Edgerun</span>
          </div>
          <div class="text-right text-sm text-muted-foreground">
            <p>© <span data-current-year /> Edgerun. All rights reserved.</p>
          </div>
        </div>
      </div>
    </footer>
  )
}
