export type DocsNavItem = {
  label: string
  href: string
}

export function getDocsNav(version = 'main'): DocsNavItem[] {
  return [
    { label: 'Overview', href: '/docs/' },
    { label: 'Get Started Guide', href: '/docs/getting-started/quick-start/' },
    { label: 'Address Generator CLI', href: `/docs/${version}/address-generator-cli.html` },
    { label: 'Address Generator Payload', href: `/docs/${version}/address-generator-payload.html` },
    { label: 'API Reference', href: `/docs/${version}/api-reference.html` },
    { label: 'Whitepaper', href: `/docs/${version}/Whitepaper.html` },
    { label: 'Scheduler API', href: `/docs/${version}/scheduler-api.html` },
    { label: 'Changelog', href: `/docs/${version}/changelog.html` },
    { label: 'Version Index', href: `/docs/${version}/` }
  ]
}
