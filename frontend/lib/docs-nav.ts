// SPDX-License-Identifier: Apache-2.0
import {
  docsAddressGeneratorCliHref,
  docsAddressGeneratorPayloadHref,
  docsApiReferenceHref,
  docsChangelogHref,
  docsOverviewHref,
  docsQuickStartHref,
  docsSchedulerApiHref,
  docsVersionIndexHref,
  docsWhitepaperHref
} from './docs-links'

export type DocsNavItem = {
  label: string
  href: string
}

export function getDocsNav(version = 'main'): DocsNavItem[] {
  return [
    { label: 'Overview', href: docsOverviewHref() },
    { label: 'Get Started Guide', href: docsQuickStartHref() },
    { label: 'Address Generator CLI', href: docsAddressGeneratorCliHref(version) },
    { label: 'Address Generator Payload', href: docsAddressGeneratorPayloadHref(version) },
    { label: 'API Reference', href: docsApiReferenceHref(version) },
    { label: 'Whitepaper', href: docsWhitepaperHref(version) },
    { label: 'Scheduler API', href: docsSchedulerApiHref(version) },
    { label: 'Changelog', href: docsChangelogHref(version) },
    { label: 'Version Index', href: docsVersionIndexHref(version) }
  ]
}
