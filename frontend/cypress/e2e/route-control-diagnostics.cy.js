// SPDX-License-Identifier: Apache-2.0
describe('route control diagnostics', () => {
  it('does not render removed route diagnostics blocks in nav', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        win.__EDGERUN_API_BASE = 'https://api.edgerun.tech'
        win.localStorage.setItem('edgerun.route.controlBase', 'https://storage.edgerun.tech')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('[data-testid="route-debug-rail"]').should('not.exist')
    cy.get('[data-testid="route-debug-scheduler"]').should('not.exist')
    cy.get('[data-testid="route-debug-control-ws"]').should('not.exist')
    cy.get('[data-testid="route-debug-overlay-ws"]').should('not.exist')
    cy.get('[data-testid="route-debug-overlay-summary"]').should('not.exist')
    cy.get('[data-testid="route-debug-route-advert"]').should('not.exist')
    cy.get('link[data-edgerun-dynamic-favicon]')
      .should('have.attr', 'type', 'image/svg+xml')
      .invoke('attr', 'href')
      .should('match', /^data:image\/svg\+xml,/)
  })

  it('keeps nav functional when configured API base is invalid', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        win.__EDGERUN_API_BASE = 'nota://url'
        win.localStorage.setItem('edgerun.route.controlBase', 'https://storage.edgerun.tech')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('a[data-nav-link]').contains('Run Job').should('be.visible')
    cy.get('[aria-label="Open terminal drawer"], [aria-label="Connect wallet to use terminal drawer"]').should('exist')
  })
})
