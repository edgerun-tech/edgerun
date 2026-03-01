// SPDX-License-Identifier: Apache-2.0

describe('terminal term-web embed', () => {
  it('opens non-route targets with term-web iframe surface', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        try {
          win.indexedDB.deleteDatabase('edgerun-frontend-ui')
        } catch {
          // ignore cleanup errors
        }
        win.localStorage.setItem('edgerun.wallet.session.v1', JSON.stringify({
          connected: true,
          address: 'Cypresstest111111111111111111111111111111',
          provider: 'cypress'
        }))
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('button[aria-controls="edgerun-terminal-drawer"]').first().click({ force: true })
    cy.get('#edgerun-terminal-drawer', { timeout: 10000 }).should('be.visible')

    cy.get('[data-testid="terminal-device-name-input"]').clear().type('Term Web Local')
    cy.get('[data-testid="terminal-device-url-input"]').clear().type('http://127.0.0.1:5577')
    cy.contains('button', /^Add Device$/).click({ force: true })
    cy.contains('p', 'http://127.0.0.1:5577').should('exist')
    cy.contains('p', 'Term Web Local')
      .parents('.rounded-md.border')
      .first()
      .contains('button', /^Connect$/)
      .click({ force: true })

    cy.get('[data-testid="terminal-web-iframe"]', { timeout: 10000 })
      .should('be.visible')
      .should('have.attr', 'src')
      .and('include', '/term?sid=')

    cy.get('[data-testid="terminal-nonroute-disabled"]').should('not.exist')
  })
})
