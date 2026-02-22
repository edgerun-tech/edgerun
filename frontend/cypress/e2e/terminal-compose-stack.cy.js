// SPDX-License-Identifier: Apache-2.0
describe('terminal docker compose stack', () => {
  it('connects route target from compose stack and renders routed in-app terminal pane', () => {
    const schedulerPort = 8090
    const termServerPort = 8081
    const schedulerBase = `http://127.0.0.1:${schedulerPort}`
    const termServerBase = `http://127.0.0.1:${termServerPort}`
    let deviceId = ''

    cy.request(`${schedulerBase}/health`)
      .its('status')
      .should('eq', 200)

    cy.request(`${termServerBase}/v1/device/identity`).then((identityResp) => {
      expect(identityResp.status).to.eq(200)
      deviceId = String(identityResp.body?.device_pubkey_b64url || '')
      expect(deviceId).to.be.a('string')
      expect(deviceId.length).to.be.greaterThan(0)

      cy.request(`${schedulerBase}/v1/route/resolve/${encodeURIComponent(deviceId)}`).then((resolveResp) => {
        expect(resolveResp.status).to.eq(200)
        expect(resolveResp.body?.ok).to.eq(true)
        expect(resolveResp.body?.found).to.eq(true)
      })
    })

    cy.then(() => {
      const activeDeviceId = deviceId
      expect(activeDeviceId).to.be.a('string').and.have.length.greaterThan(0)

      cy.visit('/', {
        onBeforeLoad(win) {
          const provider = {
            isPhantom: true,
            isConnected: true,
            publicKey: { toString: () => activeDeviceId },
            connect: () => Promise.resolve({ publicKey: { toString: () => activeDeviceId } }),
            disconnect: () => Promise.resolve(),
            on: () => {},
            removeListener: () => {}
          }
          win.solana = provider
          win.phantom = { solana: provider }
          win.localStorage.setItem('edgerun.wallet.session.v1', JSON.stringify({
            connected: true,
            address: activeDeviceId,
            provider: 'cypress'
          }))
          win.localStorage.setItem('edgerun.route.controlBase', schedulerBase)
        }
      })

      cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

      cy.get('button[aria-controls="edgerun-terminal-drawer"]')
        .first()
        .should('not.be.disabled')
        .click({ force: true })
      cy.get('#edgerun-terminal-drawer', { timeout: 10000 }).should('be.visible')

      cy.get('input[placeholder="Device name"]').clear().type('Compose Routed Device')
      cy.get('input[placeholder="route://device-id"]').clear().type(`route://${activeDeviceId}`)
      cy.contains('button', /^Add Device$/).click({ force: true })
      cy.contains('p', 'Compose Routed Device')
        .parents('.rounded-md.border')
        .first()
        .contains('button', /^Connect$/)
        .click({ force: true })

      cy.get('#edgerun-terminal-drawer iframe').should('not.exist')
      cy.get('[data-testid="routed-terminal-log"]', { timeout: 10000 }).should('exist')
      cy.get('#edgerun-terminal-drawer').contains(`route://${activeDeviceId} attached`, { timeout: 10000 }).should('exist')
    })
  })
})
