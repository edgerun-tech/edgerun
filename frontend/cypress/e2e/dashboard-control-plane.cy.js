// SPDX-License-Identifier: Apache-2.0
describe('dashboard control plane health', () => {
  it('renders control-plane connectivity panel with resolved values', () => {
    cy.visit('/dashboard/index.html', {
      onBeforeLoad(win) {
        win.__EDGERUN_ROUTE_CONTROL_PROBE_MOCK_ENABLED__ = true
        win.__EDGERUN_ROUTE_CONTROL_PROBE_MOCK__ = ({ base, source }) => ({
          base,
          source,
          checkedAt: Date.parse('2026-02-24T12:34:56Z'),
          controlWsReachable: true,
          controlWsLatencyMs: 42,
          error: ''
        })
      }
    })
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="dashboard-control-plane-card"]').should('be.visible')
    cy.get('[data-control-field="controlBase"]').should('not.contain.text', 'loading...')
    cy.get('[data-control-field="controlWs"]').should('contain.text', 'reachable')
    cy.get('[data-control-field="controlWsLatency"]').should('contain.text', '42 ms')
    cy.get('[data-control-field="controlCheckedAt"]').should('not.contain.text', 'loading...')
  })
})
