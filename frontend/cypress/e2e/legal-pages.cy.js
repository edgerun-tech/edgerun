// SPDX-License-Identifier: Apache-2.0
describe('legal pages', () => {
  it('renders privacy policy content instead of placeholder state', () => {
    cy.visit('/legal/privacy/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('h1', 'Privacy Policy').should('be.visible')
    cy.contains('Effective February 24, 2026').should('be.visible')
    cy.contains('h3', 'Data We Collect').should('be.visible')
    cy.contains('h3', 'How We Use Data').should('be.visible')
    cy.contains('Policy Publication').should('not.exist')
    cy.contains('This page will be replaced by the generated privacy policy during release publishing.').should('not.exist')
  })

  it('renders terms of service content instead of placeholder state', () => {
    cy.visit('/legal/terms/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('h1', 'Terms of Service').should('be.visible')
    cy.contains('Effective February 24, 2026').should('be.visible')
    cy.contains('h3', 'On-Chain and Financial Risk').should('be.visible')
    cy.contains('h3', 'Limitation of Liability').should('be.visible')
    cy.contains('Terms Publication').should('not.exist')
    cy.contains('This placeholder is replaced with generated terms content during release builds.').should('not.exist')
  })

  it('renders sla content instead of placeholder state', () => {
    cy.visit('/legal/sla/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('h1', 'Service Level Agreement').should('be.visible')
    cy.contains('Effective February 24, 2026').should('be.visible')
    cy.contains('h3', 'Availability Target').should('be.visible')
    cy.contains('h3', 'Incident Severity Targets').should('be.visible')
    cy.contains('SLA Publication').should('not.exist')
    cy.contains('This page is reserved for generated SLA definitions and support windows.').should('not.exist')
  })
})
