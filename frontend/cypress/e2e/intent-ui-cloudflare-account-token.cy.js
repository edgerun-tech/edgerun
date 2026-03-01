// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui cloudflare account token', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_cloudflare_account_token')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('verifies cloudflare account api token and links integration', () => {
    cy.intercept('POST', '/api/cloudflare/verify', {
      statusCode: 200,
      body: {
        ok: true,
        status: 'active',
        token_id: 'cf-token-id-1'
      }
    }).as('cfVerify')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('cloudflare_token')
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-mode-cloudflare"]').should('contain.text', 'User-owned')
    cy.get('[data-testid="provider-open-cloudflare"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-cloudflare"]').should('be.visible')
    cy.contains('Cloudflare account API token').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="provider-token-cloudflare"]').type('cf_test_account_api_token_123456')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-verify-cloudflare"]').click({ force: true })
    cy.wait('@cfVerify')
    cy.get('[data-testid="integration-stepper-success"]').should('exist')
    cy.get('[data-testid="provider-save-cloudflare"]').click({ force: true })
    cy.contains('Cloudflare integration linked.').should('be.visible')
  })
})
