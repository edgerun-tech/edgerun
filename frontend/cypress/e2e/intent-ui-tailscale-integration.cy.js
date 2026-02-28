// SPDX-License-Identifier: Apache-2.0

describe('intent ui tailscale integration', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_tailscale_test')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows tailscale quickstart and allows linking integration', () => {
    cy.intercept('POST', '/api/tailscale/devices', {
      statusCode: 200,
      body: {
        ok: true,
        error: '',
        devices: [
          {
            id: 'n1',
            hostname: 'edge-node-1',
            advertisedRoutes: ['10.0.0.0/16'],
            enabledRoutes: []
          }
        ]
      }
    }).as('tailscaleDevices')

    cy.intercept('POST', '/api/tailscale/device-routes', {
      statusCode: 200,
      body: {
        ok: true,
        error: '',
        advertisedRoutes: ['10.0.0.0/16'],
        enabledRoutes: ['10.0.0.0/16']
      }
    }).as('tailscaleSetRoutes')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('tailscale_auth_key')
        win.localStorage.removeItem('tailscale_api_key')
        win.localStorage.removeItem('tailscale_tailnet')
        seedProfileSession(win)
      }
    })

    cy.window().then((win) => {
      expect(typeof win.__intentDebug?.openWindow).to.eq('function')
      win.__intentDebug.openWindow('integrations')
    })

    cy.get('[data-testid="provider-connected-tailscale"]').should('contain.text', 'Not connected')
    cy.get('[data-testid="provider-mode-tailscale"]').should('contain.text', 'Platform')

    cy.get('[data-testid="provider-open-tailscale"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-tailscale"]').should('be.visible')
    cy.get('[data-testid="provider-tailscale-quickstart"]').should('be.visible')
    cy.get('[data-testid="tailscale-up-command"]').should('contain.text', '--advertise-connector')
    cy.get('[data-testid="tailscale-policy-snippet"]').should('contain.text', 'tailscale.com/app-connectors')

    cy.get('[data-testid="tailscale-connector-tag-input"]').clear().type('tag:edge-app')
    cy.get('[data-testid="tailscale-app-domains-input"]').clear().type('os.edgerun.tech,api.edgerun.tech')
    cy.get('[data-testid="tailscale-up-command"]').should('contain.text', '--advertise-tags=tag:edge-app')
    cy.get('[data-testid="tailscale-policy-snippet"]').should('contain.text', 'api.edgerun.tech')
    cy.get('[data-testid="tailscale-api-key-input"]').type('tskey-api-testkey')
    cy.get('[data-testid="tailscale-tailnet-input"]').type('acme.github')
    cy.get('[data-testid="tailscale-load-devices"]').click({ force: true })
    cy.wait('@tailscaleDevices')
    cy.get('[data-testid="tailscale-device-select"]').should('contain.text', 'edge-node-1')
    cy.get('[data-testid="tailscale-routes-input"]').scrollIntoView().clear({ force: true }).type('10.0.0.0/16', { force: true })
    cy.get('[data-testid="tailscale-apply-routes"]').click({ force: true })
    cy.wait('@tailscaleSetRoutes')
    cy.get('[data-testid="tailscale-selected-device-routes"]').should('contain.text', '10.0.0.0/16')

    cy.get('[data-testid="provider-save-tailscale"]').click({ force: true })

    cy.get('[data-testid="provider-connected-tailscale"]').should('contain.text', 'Connected')
    cy.get('[data-testid="provider-available-tailscale"]').should('contain.text', 'Available')
    cy.get('[data-testid="provider-mode-tailscale"]').should('contain.text', 'Platform')
  })
})
