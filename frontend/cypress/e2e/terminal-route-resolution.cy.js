// SPDX-License-Identifier: Apache-2.0
describe('terminal route device resolution', () => {
  it('renders routed in-app terminal pane for route:// targets', () => {
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
    cy.get('input[placeholder="https://device.edgerun.tech"]').clear().type('route://deadbeef')
    cy.contains('button', /^Add Device$/).click({ force: true })
    cy.contains('p', 'Broken Route Device')
      .parents('.rounded-md.border')
      .first()
      .contains('button', /^Connect$/)
      .click({ force: true })

    cy.get('[data-testid="routed-terminal-log"]').should('exist')
    cy.get('#edgerun-terminal-drawer').contains('route://deadbeef attached').should('exist')
    cy.get('iframe[src^="route://"]').should('not.exist')
  })
})
