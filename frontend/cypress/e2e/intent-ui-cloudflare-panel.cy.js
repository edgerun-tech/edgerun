// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui cloudflare panel', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_cloudflare_panel')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify([
        'openid',
        'profile',
        'edgerun:profile.read',
        'edgerun:profile.write',
        'edgerun:intents.submit',
        'edgerun:cap.network.use'
      ])
    )
  }

  it('loads cloudflare domains, tunnels, access, workers, pages, and upserts dns record', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.setItem('cloudflare_token', 'cf_test_account_api_token_cloudflare_panel_123456')
        win.localStorage.setItem('intent-ui-integrations-v1', JSON.stringify({
          cloudflare: {
            connected: true,
            linked: true,
            connectorMode: 'user_owned',
            authMethod: 'token',
            capabilities: ['zones.read', 'workers.read', 'workers.write'],
            connectedAt: new Date().toISOString(),
            accountLabel: 'Cloudflare Account'
          }
        }))
        seedProfileSession(win)
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.window().then((win) => {
      win.__intentDebug.openWindow('cloudflare')
    })

    cy.contains('Cloudflare').should('be.visible')
    cy.get('[data-testid="cloudflare-panel"]', { timeout: 15000 }).should('exist')
    cy.get('[data-testid="cloudflare-zones-list"]').should('contain.text', 'example.com')
    cy.get('[data-testid="cloudflare-tunnels-list"]').should('contain.text', 'edge-terminal')
    cy.get('[data-testid="cloudflare-access-list"]').should('contain.text', 'Terminal Access')
    cy.get('[data-testid="cloudflare-workers-list"]').should('contain.text', 'edge-worker')
    cy.get('[data-testid="cloudflare-pages-list"]').should('contain.text', 'edge-site')
    cy.get('[data-testid="cloudflare-zone-select"]').select('example.com', { force: true })

    cy.get('[data-testid="cloudflare-dns-records-list"]').within(() => {
      cy.get('input[placeholder="Name (e.g. app.example.com)"]').clear({ force: true }).type('app.example.com', { force: true })
      cy.get('input[placeholder="Content (IP, hostname, or TXT value)"]').clear({ force: true }).type('service.example.net', { force: true })
      cy.get('[data-testid="cloudflare-dns-upsert-submit"]').click({ force: true })
    })

    cy.get('[data-testid="cloudflare-panel-notice"]').should('contain.text', 'DNS record')
    cy.get('[data-testid="cloudflare-dns-records-list"]').should('contain.text', 'app.example.com')
  })
})
