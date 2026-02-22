describe('terminal user requests use a single control channel', () => {
  it('does not issue legacy HTTP route resolve requests on connect', () => {
    cy.intercept('GET', '**/v1/route/resolve/*', {
      statusCode: 200,
      body: { ok: true, found: false }
    }).as('routeResolve')

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
    cy.get('button[aria-controls="edgerun-terminal-drawer"]').first().click({ force: true })
    cy.get('#edgerun-terminal-drawer', { timeout: 10000 }).should('be.visible')

    cy.get('input[placeholder="Device name"]').clear().type('Single Channel Device')
    cy.get('input[placeholder="https://device.edgerun.tech"]').clear().type('route://single-channel')
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
})
