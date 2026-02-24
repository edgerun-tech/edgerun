// SPDX-License-Identifier: Apache-2.0

describe('footer lead form', () => {
  it('submits email collection request and shows success feedback', () => {
    cy.intercept('POST', '/api/lead', (request) => {
      expect(request.body).to.deep.include({
        email: 'ops@edgerun.dev',
        sourcePath: '/'
      })
      request.reply({
        statusCode: 202,
        body: { ok: true }
      })
    }).as('leadSubmit')

    cy.visit('/')
    cy.get('[data-testid="footer-lead-email"]').first().clear({ force: true })
    cy.get('[data-testid="footer-lead-email"]').first().type('ops@edgerun.dev', { force: true })
    cy.get('[data-testid="footer-lead-submit"]').first().click({ force: true })
    cy.wait('@leadSubmit')
    cy.get('[data-testid="footer-lead-feedback"]').contains('Thanks. You are on the release updates list.').should('be.visible')
  })
})
