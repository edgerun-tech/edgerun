// SPDX-License-Identifier: Apache-2.0
describe('dashboard control plane health', () => {
  it('renders control-plane connectivity panel with resolved values', () => {
    cy.visit('/dashboard/index.html')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="dashboard-control-plane-card"]').should('be.visible')
    cy.get('[data-control-field="controlBase"]').should('not.contain.text', 'loading...')
    cy.get('[data-control-field="controlWs"]').should('not.contain.text', 'loading...')
    cy.get('[data-control-field="controlWsLatency"]').should('not.contain.text', 'loading...')
    cy.get('[data-control-field="controlCheckedAt"]').should('not.contain.text', 'loading...')
  })
})
