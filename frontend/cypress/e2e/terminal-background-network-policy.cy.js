// SPDX-License-Identifier: Apache-2.0
describe('terminal background network policy', () => {
  it('does not issue background health or device polling requests while idle', () => {
    cy.intercept('GET', '**/v1/device/identity').as('deviceIdentity')
    cy.intercept('GET', '**/health').as('health')

    cy.visit('/', {
      onBeforeLoad(win) {
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

    cy.wait(13000)

    cy.get('@deviceIdentity.all').then((calls) => {
      expect(calls.length).to.equal(0)
    })
    cy.get('@health.all').then((calls) => {
      expect(calls.length).to.equal(0)
    })
  })
})
