// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

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
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('tailscale_auth_key')
        win.localStorage.removeItem('tailscale_api_key')
        win.localStorage.removeItem('tailscale_tailnet')
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })

    cy.get('[data-testid="provider-open-tailscale"]').should('exist')

    cy.get('[data-testid="provider-connected-tailscale"]').should('contain.text', 'Not connected')
    cy.get('[data-testid="provider-mode-tailscale"]').should('contain.text', 'User-owned')

    cy.get('[data-testid="provider-open-tailscale"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-tailscale"]').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="tailscale-api-key-input"]').type('tskey-api-testkey')
    cy.get('[data-testid="tailscale-tailnet-input"]').type('acme.github')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-tailscale-quickstart"]').should('be.visible')
    cy.get('[data-testid="tailscale-load-devices"]').click({ force: true })
    cy.wait('@tailscaleDevices')
    cy.get('[data-testid="integration-stepper-success"]').should('be.visible')
    cy.get('[data-testid="provider-save-tailscale"]').click({ force: true })

    cy.contains('Tailscale integration linked.').should('be.visible')
    cy.get('[data-testid="provider-mode-tailscale"]').should('contain.text', 'User-owned')
  })
})
