// SPDX-License-Identifier: Apache-2.0

describe('intent ui matrix token guidance', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_matrix_token_guidance')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows where to get matrix bridge token for telegram', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-open-telegram"]').click({ force: true })

    cy.get('[data-testid="matrix-token-guidance-telegram"]').should('be.visible')
    cy.get('[data-testid="matrix-token-auto-telegram"]').should('be.visible')
    cy.contains('We still use the bridge provisioning/API secret').should('be.visible')
    cy.contains('This is not your Matrix account password or OAuth token.').should('be.visible')
    cy.contains('Open setup docs').should('not.exist')
  })
})
