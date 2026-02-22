// SPDX-License-Identifier: Apache-2.0
describe('terminal docker compose stack', () => {
  it('resolves term-server route via scheduler and blocks legacy iframe terminal embedding', () => {
    let deviceId = ''

    cy.request('http://127.0.0.1:8090/health')
      .its('status')
      .should('eq', 200)

    cy.request('http://127.0.0.1:8080/v1/device/identity').then((identityResp) => {
      expect(identityResp.status).to.eq(200)
      deviceId = String(identityResp.body?.device_pubkey_b64url || '')
      expect(deviceId).to.be.a('string')
      expect(deviceId.length).to.be.greaterThan(0)

      cy.request(`http://127.0.0.1:8090/v1/route/resolve/${encodeURIComponent(deviceId)}`).then((resolveResp) => {
        expect(resolveResp.status).to.eq(200)
        expect(resolveResp.body?.ok).to.eq(true)
        expect(resolveResp.body?.found).to.eq(true)
      })
    })

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
        win.localStorage.setItem('edgerun.route.controlBase', 'http://127.0.0.1:8090')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('button[aria-controls="edgerun-terminal-drawer"]')
      .first()
      .should('not.be.disabled')
      .click({ force: true })
    cy.get('#edgerun-terminal-drawer', { timeout: 10000 }).should('be.visible')

    cy.get('input[placeholder="Device name"]').clear().type('Compose Routed Device')
    cy.get('input[placeholder="route://device-id"]').clear().type(`route://${deviceId}`)
    cy.contains('button', /^Add Device$/).click({ force: true })

    cy.contains('p', 'Compose Routed Device')
      .parents('.rounded-md.border')
      .first()
      .contains('button', /^Connect$/)
      .click({ force: true })

    cy.get('#edgerun-terminal-drawer iframe').should('not.exist')
    cy.get('[data-testid="routed-terminal-log"]', { timeout: 10000 }).should('exist')
    cy.get('#edgerun-terminal-drawer').contains(`route://${deviceId} attached`).should('exist')
  })
})
