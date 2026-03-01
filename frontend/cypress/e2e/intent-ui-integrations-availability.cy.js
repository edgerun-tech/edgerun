// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui integrations connection truth', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_truth_test')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('keeps github user-owned and marks it available after PAT token linking', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('github_token')
        win.localStorage.removeItem('google_token')
        win.localStorage.removeItem('cloudflare_token')
        win.localStorage.removeItem('vercel_token')
        win.localStorage.removeItem('hetzner_token')
        seedProfileSession(win)
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.get('button[title="Integrations panel"]').first().click({ force: true })

    cy.get('[data-testid="provider-open-github"]', { timeout: 10000 }).should('exist')

    cy.get('[data-testid="provider-mode-github"]').should('contain.text', 'User-owned')
    cy.get('[data-testid="provider-connected-github"]').should('contain.text', 'Not connected')
    cy.get('[data-testid="provider-available-github"]').should('contain.text', 'Unavailable')

    cy.get('[data-testid="provider-open-github"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-github"]').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="provider-token-github"]').type('ghp_test_token_for_intent_ui')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-save-github"]').click({ force: true })

    cy.get('[data-testid="provider-connected-github"]').should('contain.text', 'Connected')
  })

  it('writes github token to local credentials vault and remains connected across revisit', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('github_token')
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-open-github"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-github"]').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="provider-token-github"]').type('ghp_test_token_for_persistence')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-save-github"]').click({ force: true })

    cy.window().then((win) => {
      const raw = String(win.localStorage.getItem('intent-ui-local-bridge-credentials-sim-v1') || '[]')
      const parsed = JSON.parse(raw)
      const tokenEntry = parsed.find((entry) => String(entry?.name || '').trim() === 'integration/github/token')
      expect(tokenEntry).to.not.equal(undefined)
      expect(String(tokenEntry?.secret || '')).to.eq('ghp_test_token_for_persistence')
    })

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-connected-github"]').should('contain.text', 'Connected')
  })

  it('keeps beeper user-owned and marks connected after desktop api verification/link', () => {
    cy.intercept('POST', '/api/beeper/verify', {
      statusCode: 200,
      body: {
        ok: true,
        account_count: 5,
        accounts: { items: [] }
      }
    }).as('beeperVerify')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('beeper_access_token')
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-mode-beeper"]').should('contain.text', 'User-owned')
    cy.get('[data-testid="provider-connected-beeper"]').should('contain.text', 'Not connected')

    cy.get('[data-testid="provider-open-beeper"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-beeper"]').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="provider-token-beeper"]').type('beeper_access_token_test_12345678')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-verify-beeper"]').click({ force: true })
    cy.wait('@beeperVerify')
    cy.get('[data-testid="integration-stepper-success"]').should('exist')
    cy.get('[data-testid="provider-save-beeper"]').click({ force: true })

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        seedProfileSession(win)
      }
    })
    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-connected-beeper"]').should('contain.text', 'Connected')
  })
})
