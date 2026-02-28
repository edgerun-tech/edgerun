// SPDX-License-Identifier: Apache-2.0

describe('intent ui integrations ownership mode', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_test_owner')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('keeps github in user-owned mode and removes mode step from the dialog', () => {
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
    cy.get('[data-testid="provider-open-github"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-github"]').should('be.visible')

    cy.get('[data-testid="integration-step-1"]').should('not.exist')
    cy.get('[data-testid="integration-step-2"]').should('contain.text', '1. Values')
    cy.get('[data-testid="provider-ownership-platform-github"]').should('not.exist')
    cy.contains('button', 'Close').click({ force: true })

    cy.get('[data-testid="provider-mode-github"]').should('contain.text', 'User-owned')
  })
})
