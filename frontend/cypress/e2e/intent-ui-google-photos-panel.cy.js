// SPDX-License-Identifier: Apache-2.0

describe('intent ui google photos panel surfacing', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_google_photos_panel')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('opens photos from quick action and command', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.setItem('google_token', 'test-google-token')
      }
    })

    cy.get('button[title="Photos"]').first().click({ force: true })
    cy.get('[aria-label="Photos window"]').should('exist')
    cy.contains('https://photos.google.com').should('exist')

    cy.get('input[type="text"]').first().clear().type('google photos{enter}', { force: true })
    cy.get('[aria-label="Photos window"]').should('exist')
  })
})
