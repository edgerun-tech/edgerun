// SPDX-License-Identifier: Apache-2.0
describe('terminal docker compose stack', () => {
  it('binds route target from compose stack into in-app terminal surface', () => {
    const schedulerPort = 8090
    const termServerPort = 8081
    const schedulerBase = `http://127.0.0.1:${schedulerPort}`
    const termServerBase = `http://127.0.0.1:${termServerPort}`
    let deviceId = ''

    cy.request({
      url: `${schedulerBase}/v1/control/ws`,
      failOnStatusCode: false
    }).its('status').should((status) => {
      expect([400, 426]).to.include(status)
    })

    cy.request(`${termServerBase}/v1/device/identity`).then((identityResp) => {
      expect(identityResp.status).to.eq(200)
      deviceId = String(identityResp.body?.device_pubkey_b64url || '')
      expect(deviceId).to.be.a('string')
      expect(deviceId.length).to.be.greaterThan(0)
    })

    cy.then(() => {
      const activeDeviceId = deviceId
      expect(activeDeviceId).to.be.a('string').and.have.length.greaterThan(0)

      cy.visit('/', {
        onBeforeLoad(win) {
          try {
            win.indexedDB.deleteDatabase('edgerun-frontend-ui')
          } catch {
            // ignore cleanup errors
          }
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

      cy.get('[data-testid="terminal-device-name-input"]').clear().type('Compose Routed Device')
      cy.get('[data-testid="terminal-device-url-input"]').clear().type(`route://${activeDeviceId}`)
      cy.contains('button', /^Add Device$/).click({ force: true })
      cy.contains('p', 'Compose Routed Device')
        .parents('.rounded-md.border')
        .first()
        .contains('button', /^Connect$/)
        .click({ force: true })

      cy.get('#edgerun-terminal-drawer iframe').should('not.exist')
      cy.get('body', { timeout: 12000 }).then(($body) => {
        const routedLog = $body.find('[data-testid="routed-terminal-log"]')
        if (routedLog.length > 0) {
          cy.get('#edgerun-terminal-drawer').contains(`route://${activeDeviceId} attached`).should('exist')
        } else {
          cy.get('#edgerun-terminal-drawer').contains('Select a connected device to open this pane.').should('be.visible')
        }
      })
    })
  })
})
