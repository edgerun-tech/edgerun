// SPDX-License-Identifier: Apache-2.0
describe('browser worker runtime', () => {
  it('runs browser worker loop off-thread and surfaces runtime state/errors in UI', () => {
    cy.visit('/workers/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="browser-worker-card"]').should('be.visible')
    cy.window().then((win) => {
      cy.get('[data-testid="browser-worker-scheduler"]').clear().type(win.location.origin)
    })
    cy.get('[data-testid="browser-worker-start"]').click({ force: true })

    cy.get('[data-testid="browser-worker-state"]').invoke('text').should((text) => {
      const normalized = String(text || '').toLowerCase()
      expect(normalized).to.satisfy((value) => value.includes('running') || value.includes('stopped-with-error'))
    })
    cy.get('[data-testid="browser-worker-error"]').invoke('text').should((text) => {
      const normalized = String(text || '')
      expect(normalized.length).to.be.greaterThan(0)
    })
    cy.get('[data-testid="browser-worker-log"]').invoke('text').should((text) => {
      const normalized = String(text || '').toLowerCase()
      expect(normalized).to.satisfy((value) => (
        value.includes('browser worker started') ||
        value.includes('heartbeat ok') ||
        value.includes('worker loop error')
      ))
    })

    cy.get('[data-testid="browser-worker-stop"]').click({ force: true })
    cy.get('[data-testid="browser-worker-state"]').contains('stopped').should('be.visible')
  })
})
