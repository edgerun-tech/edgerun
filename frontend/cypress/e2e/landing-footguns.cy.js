// SPDX-License-Identifier: Apache-2.0
describe('landing footgun regressions', () => {
  it('routes hero CTAs to distinct non-doc operational flows', () => {
    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.contains('a', 'Run a Job').should('have.attr', 'href', '/run/')
    cy.contains('a', 'Become a Worker').should('have.attr', 'href', '/workers/')
  })

  it('uses distinct canonical docs links in footer resources', () => {
    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.contains('a', 'Getting Started').should('have.attr', 'href', '/docs/getting-started/quick-start/')
    cy.contains('a', 'API Reference').should('have.attr', 'href', '/docs/main/api-reference.html')
  })

  it('does not print keypair_hex in terminal demo output', () => {
    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.contains('button', 'run demo').click({ force: true })
    cy.get('#address-terminal-root', { timeout: 20000 }).should('not.contain.text', 'keypair_hex')
  })

  it('does not render unverifiable hardcoded KPI claims in hero', () => {
    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.contains('Consensus Rate').should('not.exist')
    cy.contains('Avg Settlement').should('not.exist')
    cy.contains('Jobs Executed').should('not.exist')
  })

  it('renders hero chain snapshot fields wired to chain runtime bindings', () => {
    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="hero-chain-snapshot"]').within(() => {
      cy.contains('Live Chain Snapshot')
      cy.get('[data-chain-field="cluster"]').should('exist')
      cy.get('[data-chain-field="slot"]').should('exist')
      cy.get('[data-chain-field="tps"]').should('exist')
      cy.get('[data-chain-field="blockHeight"]').should('exist')
    })
  })
})
