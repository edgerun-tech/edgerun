// SPDX-License-Identifier: Apache-2.0

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
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('github_token')
        win.localStorage.removeItem('google_token')
        win.localStorage.removeItem('cloudflare_token')
        win.localStorage.removeItem('vercel_token')
        win.localStorage.removeItem('qwen_token')
        win.localStorage.removeItem('hetzner_token')
        seedProfileSession(win)
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.window().then((win) => {
      expect(typeof win.__intentDebug?.openWindow).to.eq('function')
      win.__intentDebug.openWindow('integrations')
    })

    cy.get('[data-testid="provider-open-github"]').should('exist')

    cy.get('[data-testid="provider-mode-github"]').should('contain.text', 'User-owned')
    cy.get('[data-testid="provider-connected-github"]').should('contain.text', 'Not connected')
    cy.get('[data-testid="provider-available-github"]').should('contain.text', 'Unavailable')

    cy.get('[data-testid="provider-open-github"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-github"]').should('be.visible')
    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-token-github"]').type('ghp_test_token_for_intent_ui')
    cy.get('[data-testid="integration-step-3"]').click({ force: true })
    cy.get('[data-testid="provider-verify-github"]').click({ force: true })
    cy.get('[data-testid="integration-step-4"]').click({ force: true })
    cy.get('[data-testid="provider-save-github"]').click({ force: true })

    cy.get('[data-testid="provider-connected-github"]').should('contain.text', 'Connected')
    cy.get('[data-testid="provider-available-github"]').should('contain.text', 'Available')
  })
})
