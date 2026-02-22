// SPDX-License-Identifier: Apache-2.0
export const DOCS_DEFAULT_VERSION = 'main'

export function docsVersionIndexHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/`
}

export function docsOverviewHref(): string {
  return '/docs/'
}

export function docsQuickStartHref(): string {
  return '/docs/getting-started/quick-start/'
}

export function docsAddressGeneratorCliHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/address-generator-cli.html`
}

export function docsAddressGeneratorPayloadHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/address-generator-payload.html`
}

export function docsApiReferenceHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/api-reference.html`
}

export function docsWhitepaperHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/Whitepaper.html`
}

export function docsSchedulerApiHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/scheduler-api.html`
}

export function docsChangelogHref(version = DOCS_DEFAULT_VERSION): string {
  return `/docs/${version}/changelog.html`
}

export function docsLeafPrettyHref(version: string, slug: string): string {
  return `/docs/${version}/${slug}/`
}

