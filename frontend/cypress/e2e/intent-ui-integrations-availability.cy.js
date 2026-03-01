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

  it('keeps github user-owned and marks it available after PAT verification and linking', () => {
    cy.intercept('GET', 'https://api.github.com/user', {
      statusCode: 200,
      body: {
        login: 'octocat'
      }
    }).as('githubUser')

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

    cy.get('[data-testid="provider-open-github"]').should('exist')

    cy.get('[data-testid="provider-mode-github"]').should('contain.text', 'User-owned')
    cy.get('[data-testid="provider-connected-github"]').should('contain.text', 'Not connected')
    cy.get('[data-testid="provider-available-github"]').should('contain.text', 'Unavailable')

    cy.get('[data-testid="provider-open-github"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-github"]').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="provider-token-github"]').type('ghp_test_token_for_intent_ui')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-verify-github"]').click({ force: true })
    cy.wait('@githubUser')
    cy.get('[data-testid="integration-stepper-success"]').should('exist')
    cy.get('[data-testid="provider-save-github"]').click({ force: true })

    cy.contains('GitHub integration linked.').should('be.visible')
  })

  it('writes github token to local credentials vault and remains connected across revisit', () => {
    cy.intercept('GET', 'https://api.github.com/user', {
      statusCode: 200,
      body: {
        login: 'octocat'
      }
    }).as('githubUser')

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
    cy.get('[data-testid="provider-verify-github"]').click({ force: true })
    cy.wait('@githubUser')
    cy.get('[data-testid="integration-stepper-success"]').should('exist')
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

  it('keeps whatsapp user-owned and only marks connected after matrix runtime verification/link', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('whatsapp_token')
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-mode-whatsapp"]').should('contain.text', 'User-owned')
    cy.get('[data-testid="provider-connected-whatsapp"]').should('contain.text', 'Not connected')

    cy.get('[data-testid="provider-open-whatsapp"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-whatsapp"]').should('be.visible')
    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="provider-token-whatsapp"]').type('wa_matrix_bridge_token')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="integration-runtime-state-whatsapp"]').should('contain.text', 'Not started')
    cy.get('[data-testid="provider-verify-whatsapp"]').click({ force: true })
    cy.get('[data-testid="integration-stepper-success"]').should('be.visible')
    cy.get('[data-testid="integration-runtime-state-whatsapp"]').should('contain.text', 'Not started')
    cy.window().then((win) => {
      const raw = String(win.localStorage.getItem('intent-ui-local-bridge-mcp-sim-v1') || '{}')
      const parsed = JSON.parse(raw)
      expect(Boolean(parsed?.whatsapp?.running)).to.eq(false)
    })
    cy.get('[data-testid="provider-save-whatsapp"]').click({ force: true })

    cy.window().then((win) => {
      const raw = String(win.localStorage.getItem('intent-ui-local-bridge-mcp-sim-v1') || '{}')
      const parsed = JSON.parse(raw)
      expect(Boolean(parsed?.whatsapp?.running)).to.eq(true)
    })

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        seedProfileSession(win)
      }
    })
    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-open-whatsapp"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-whatsapp"]').should('be.visible')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="integration-runtime-state-whatsapp"]').should('contain.text', 'Running')
    cy.contains('button', 'Close').click({ force: true })
    cy.get('[data-testid="provider-connected-whatsapp"]').should('contain.text', 'Connected')
  })
})
