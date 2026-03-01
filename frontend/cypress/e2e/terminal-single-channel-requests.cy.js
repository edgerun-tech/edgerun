// SPDX-License-Identifier: Apache-2.0
describe('terminal user requests use a single control channel', () => {
  it('does not issue legacy HTTP route resolve requests on connect', () => {
    cy.intercept('GET', '**/v1/route/resolve/*', {
      statusCode: 200,
      body: { ok: true, found: false }
    }).as('routeResolve')

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
        win.localStorage.setItem('edgerun.route.controlBase', 'http://127.0.0.1:8090')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('button[aria-controls="edgerun-terminal-drawer"]').first().click({ force: true })
    cy.get('#edgerun-terminal-drawer', { timeout: 10000 }).should('be.visible')

    cy.get('[data-testid="terminal-device-name-input"]').clear().type('Single Channel Device')
    cy.get('[data-testid="terminal-device-url-input"]').clear().type('route://single-channel')
    cy.contains('button', /^Add Device$/).click({ force: true })

    let callsBeforeConnect = 0
    cy.get('@routeResolve.all').then((calls) => {
      callsBeforeConnect = calls.length
    })

    cy.contains('p', 'Single Channel Device')
      .parents('.rounded-md.border')
      .first()
      .contains('button', /^Connect$/)
      .click({ force: true })

    cy.wait(1000)

    cy.get('@routeResolve.all').then((calls) => {
      expect(calls.length - callsBeforeConnect).to.equal(0)
    })
  })

  it('uses scheduler websocket signaling and keeps terminal controls labeled', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        const NativeWebSocket = win.WebSocket
        const openedWsUrls = []
        const WrappedWebSocket = function WrappedWebSocket(url, protocols) {
          openedWsUrls.push(String(url))
          return protocols
            ? new NativeWebSocket(url, protocols)
            : new NativeWebSocket(url)
        }
        WrappedWebSocket.prototype = NativeWebSocket.prototype
        Object.setPrototypeOf(WrappedWebSocket, NativeWebSocket)
        win.WebSocket = WrappedWebSocket
        win.__openedWsUrls = openedWsUrls
        win.localStorage.setItem('edgerun.wallet.session.v1', JSON.stringify({
          connected: true,
          address: 'Cypresstest111111111111111111111111111111',
          provider: 'cypress'
        }))
        win.localStorage.setItem('edgerun.route.controlBase', 'http://127.0.0.1:8090')
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.get('input[aria-label="Demo terminal command input"]').should('exist')
    cy.wait(1500)
    cy.window().its('__openedWsUrls').then((urls) => {
      const opened = Array.isArray(urls) ? urls.map((value) => String(value)) : []
      expect(opened.some((value) => value.includes('/v1/webrtc/ws'))).to.equal(true)
    })
  })
})
