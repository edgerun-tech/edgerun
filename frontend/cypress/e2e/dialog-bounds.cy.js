// SPDX-License-Identifier: Apache-2.0
function assertDialogInViewport(title) {
  cy.contains('h2', title).should('be.visible')
  cy.contains('h2', title).closest('section').should('exist').then(($dialog) => {
    const rect = $dialog[0].getBoundingClientRect()
    expect(rect.left).to.be.at.least(0)
    expect(rect.top).to.be.at.least(0)
    expect(rect.right).to.be.at.most(Cypress.config('viewportWidth'))
    expect(rect.bottom).to.be.at.most(Cypress.config('viewportHeight'))
  })
}

describe('dialog bounds', () => {
  it('wallet dialog stays inside viewport bounds', () => {
    cy.viewport(1280, 720)
    cy.visit('/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('button[aria-label="Open wallet details"]').first().click({ force: true })
    assertDialogInViewport('Wallet + Network')
  })

  it('run mode-safety dialog stays inside viewport bounds', () => {
    cy.viewport(360, 640)
    cy.visit('/run/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('button', /^Mode Safety$/).click({ force: true })
    assertDialogInViewport('Execution Mode Safety')
  })
})
