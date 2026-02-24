// SPDX-License-Identifier: Apache-2.0
describe('economics screen', () => {
  it('renders deterministic models without placeholder pool content', () => {
    cy.visit('/token/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.contains('h1', 'SOL Economics').should('be.visible')
    cy.get('[data-testid="economics-pricing-model"]').should('contain.text', 'Deterministic Pricing Model')
    cy.get('[data-testid="economics-committee-tiers"]').should('contain.text', 'Committee Tiering by Escrow')
    cy.get('[data-testid="economics-settlement-model"]').should('contain.text', 'Settlement and Stake Lock Model')
    cy.get('[data-testid="economics-notes"]').should('contain.text', 'avoids speculative APR')

    cy.contains('POOL A').should('not.exist')
    cy.contains('POOL B').should('not.exist')
    cy.contains('POOL C').should('not.exist')
  })
})
