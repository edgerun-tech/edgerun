// SPDX-License-Identifier: Apache-2.0
describe('docs links integrity', () => {
  it('serves key docs pages and pretty URLs without 404s', () => {
    const hrefs = [
      '/docs/',
      '/docs/getting-started/quick-start/',
      '/docs/main/',
      '/docs/main/address-generator-cli.html',
      '/docs/main/address-generator-payload.html',
      '/docs/main/api-reference.html',
      '/docs/main/scheduler-api.html',
      '/docs/main/changelog.html',
      '/docs/main/Whitepaper.html',
      '/docs/main/address-generator-cli/',
      '/docs/main/address-generator-payload/',
      '/docs/main/api-reference/',
      '/docs/main/scheduler-api/'
    ]

    for (const href of hrefs) {
      cy.request(href).its('status').should('eq', 200)
    }

    cy.visit('/docs/')
    cy.contains('a', 'Address Generator CLI').should('have.attr', 'href').then((href) => {
      cy.request(String(href)).its('status').should('eq', 200)
    })
  })
})

