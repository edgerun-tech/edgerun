// SPDX-License-Identifier: Apache-2.0

describe('intent ui beeper token guidance', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_matrix_token_guidance')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows where to get beeper desktop api token', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-open-beeper"]').click({ force: true })

    cy.contains('enable Desktop API in Settings -> Developers').should('be.visible')
    cy.get('[data-testid="beeper-open-desktop-api-docs"]').should('be.visible')
  })
})
