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
    cy.intercept('GET', '/api/google/photos*', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          {
            id: 'photo-1',
            filename: 'sample-photo.jpg',
            baseUrl: 'https://example.com/photo-1',
            mediaMetadata: { creationTime: '2026-03-01T00:00:00.000Z' }
          }
        ]
      }
    }).as('googlePhotos')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.setItem('google_token', 'test-google-token')
      }
    })

    cy.get('button[title="Photos"]').first().click({ force: true })
    cy.wait('@googlePhotos')
    cy.get('[data-testid="google-photos-panel"]').should('exist')
    cy.get('[data-testid="google-photos-item"]').should('have.length.at.least', 1)
    cy.contains('sample-photo.jpg').should('be.visible')

    cy.get('input[type="text"]').first().clear().type('google photos{enter}', { force: true })
    cy.get('[data-testid="google-photos-panel"]').should('exist')
  })
})
