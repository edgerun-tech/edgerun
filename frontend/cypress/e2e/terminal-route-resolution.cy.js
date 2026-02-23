// SPDX-License-Identifier: Apache-2.0
describe('terminal route device resolution', () => {
  it('keeps route:// targets in the in-app terminal surface (never iframe fallback)', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        const provider = {
          isPhantom: true,
          isConnected: true,
          publicKey: { toString: () => 'Cypresstest111111111111111111111111111111' },
          connect: () => Promise.resolve({ publicKey: { toString: () => 'Cypresstest111111111111111111111111111111' } }),
          disconnect: () => Promise.resolve(),
          on: () => {},
          removeListener: () => {}
        }
        win.solana = provider
        win.phantom = { solana: provider }
        win.localStorage.setItem('edgerun.wallet.session.v1', JSON.stringify({
          connected: true,
          address: 'Cypresstest111111111111111111111111111111',
          provider: 'cypress'
        }))
        win.localStorage.setItem('edgerun.route.controlBase', 'http://127.0.0.1:1')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('button[aria-controls="edgerun-terminal-drawer"]')
      .first()
      .should('not.be.disabled')
      .click({ force: true })
    cy.get('#edgerun-terminal-drawer', { timeout: 10000 }).should('be.visible')

    cy.get('input[placeholder="Device name"]').clear().type('Broken Route Device')
    cy.get('input[placeholder="route://device-id"]').clear().type('route://deadbeef')
    cy.contains('button', /^Add Device$/).click({ force: true })
    cy.contains('p', 'Broken Route Device')
      .parents('.rounded-md.border')
      .first()
      .contains('button', /^Connect$/)
      .click({ force: true })

    cy.get('iframe[src^="route://"]').should('not.exist')

    cy.get('body', { timeout: 12000 }).then(($body) => {
      const routedLog = $body.find('[data-testid="routed-terminal-log"]')
      if (routedLog.length > 0) {
        cy.get('#edgerun-terminal-drawer').contains('route://deadbeef attached').should('exist')
      } else {
        cy.get('#edgerun-terminal-drawer').contains('Select a connected device to open this pane.').should('be.visible')
      }
    })
  })
})
