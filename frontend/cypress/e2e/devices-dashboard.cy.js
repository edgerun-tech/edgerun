// SPDX-License-Identifier: Apache-2.0
describe('devices dashboard', () => {
  it('renders dense demo fleet operations panels', () => {
    cy.viewport(3840, 2160)

    cy.visit('/devices/')

    cy.contains('h1', 'Devices').should('be.visible')
    cy.get('[data-testid="devices-dashboard"]').should('be.visible')
    cy.get('[data-testid="devices-kpi-card"]').should('have.length', 8)

    cy.get('[data-testid="devices-fleet-table"]').should('contain.text', 'Fleet Table')
    cy.get('[data-testid="devices-alerts"]').should('contain.text', 'Alerts')
    cy.get('[data-testid="devices-services"]').should('contain.text', 'Service Health')
    cy.get('[data-testid="devices-command-queue"]').should('contain.text', 'Command Queue')
    cy.get('[data-testid="devices-capacity-heatmap"]').should('contain.text', 'Capacity Grid')
    cy.get('[data-testid="devices-search-input"]').should('be.visible')
    cy.get('[data-testid="devices-status-filter"]').should('be.visible')
    cy.get('[data-testid="devices-bling-toggle"]').should('contain.text', 'Bling')

    cy.get('[data-testid="devices-fleet-table"] tbody tr').should('have.length.at.least', 12)
  })

  it('supports alias route and filtering controls', () => {
    cy.viewport(3840, 2160)

    cy.visit('/device/')

    cy.get('[data-testid="devices-fleet-table"] tbody tr').should('have.length.at.least', 12)
    cy.get('[data-testid="devices-status-filter"]').select('Offline')
    cy.get('[data-testid="devices-fleet-table"] tbody tr').should('have.length.at.least', 1)
    cy.get('[data-testid="devices-fleet-table"] tbody tr').each(($row) => {
      cy.wrap($row).should('contain.text', 'offline')
    })

    cy.get('[data-testid="devices-search-input"]').clear().type('edge-001')
    cy.get('[data-testid="devices-fleet-table"] tbody tr').should('have.length', 1)
    cy.get('[data-testid="devices-fleet-table"] tbody tr').first().should('contain.text', 'edge-001')
  })
})
