// SPDX-License-Identifier: Apache-2.0
describe('frontend architecture proof', () => {
  it('serves static shell, hydrates interactions, client-routes, and lazy-loads chunks', () => {
    cy.viewport(1280, 720)

    cy.request('/').its('body').then((html) => {
      expect(html).to.include('id="edgerun-root"')
      expect(html).to.include('type="module" src="/assets/client.js"')
    })

    cy.intercept('GET', '/assets/chunks/*.js').as('chunk')

    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.window().then((win) => {
      win.__edgerunBeforeUnloadCount = 0
      win.addEventListener('beforeunload', () => {
        win.__edgerunBeforeUnloadCount += 1
      })
    })

    cy.get('button[aria-label="Open wallet details"]').first().click({ force: true })
    cy.contains('h2', 'Wallet + Network').should('be.visible')
    cy.contains('button', /^Close$/).click({ force: true })

    cy.contains('a', /^Docs$/).first().click()

    cy.url().should('include', '/docs/')
    cy.window().its('__edgerunBeforeUnloadCount').should('eq', 0)

    cy.wait('@chunk')
    cy.get('[data-docs-search-input]:visible').should('exist')
    cy.get('[data-docs-search-input]:visible').clear().type('edgerun')
    cy.get('[data-docs-search-results]').should('have.attr', 'role', 'status')
    cy.get('[data-docs-search-results]').should('have.attr', 'aria-live', 'polite')
    cy.get('[data-docs-search-input]:visible').clear().type('zzzz-no-match')
    cy.get('[data-docs-search-results]').should('contain.text', 'No matching docs yet.')
  })
})
