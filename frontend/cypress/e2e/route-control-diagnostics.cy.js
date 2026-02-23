// SPDX-License-Identifier: Apache-2.0
describe('route control diagnostics', () => {
  it('shows configured control base source and overlay/scheduler status in nav', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        win.__EDGERUN_API_BASE = 'https://api.edgerun.tech'
        win.localStorage.setItem('edgerun.route.controlBase', 'https://storage.edgerun.tech')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('[data-testid="route-debug-rail"]').should('be.visible')
    cy.get('[data-testid="route-debug-scheduler"]', { timeout: 12000 }).should('contain.text', 'scheduler')
    cy.get('[data-testid="route-debug-scheduler"]').should('have.attr', 'title').and('include', 'overlay signal')
    cy.get('[data-testid="route-debug-control-ws"]').should('contain.text', 'ws')
    cy.get('[data-testid="route-debug-overlay-ws"]').should('contain.text', 'overlay-ws')
    cy.get('[data-testid="route-debug-overlay-summary"]').should('contain.text', 'overlay')
    cy.get('[data-testid="route-debug-route-advert"]').should('contain.text', 'route-advert')
  })

  it('falls back away from configured source when configured API base is invalid', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        win.__EDGERUN_API_BASE = 'nota://url'
        win.localStorage.setItem('edgerun.route.controlBase', 'https://storage.edgerun.tech')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('[data-testid="route-debug-scheduler"]', { timeout: 12000 }).invoke('text').then((value) => {
      expect(value).to.match(/scheduler (online|offline)/)
    })
    cy.get('[data-testid="route-debug-control-ws"]').invoke('text').should((value) => {
      const normalized = String(value).trim()
      expect(normalized === 'ws ok' || normalized === 'ws down').to.eq(true)
    })
  })
})
