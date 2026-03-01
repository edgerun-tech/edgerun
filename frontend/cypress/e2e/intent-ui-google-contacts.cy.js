// SPDX-License-Identifier: Apache-2.0

describe('intent ui google contacts', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_google_contacts')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('renders People API contacts in the conversations contacts tab', () => {
    cy.intercept('GET', '/api/google/contacts*', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          {
            resourceName: 'people/c123',
            names: [{ displayName: 'Ada Lovelace' }],
            emailAddresses: [{ value: 'ada@example.com' }]
          },
          {
            resourceName: 'people/c456',
            names: [{ displayName: 'Grace Hopper' }],
            emailAddresses: [{ value: 'grace@example.com' }]
          }
        ]
      }
    }).as('googleContacts')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.setItem('google_token', 'google_token_for_contacts_test')
      }
    })

    cy.wait('@googleContacts')
    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.contains('button', 'Contacts').click({ force: true })

    cy.contains('Ada Lovelace').should('be.visible')
    cy.contains('ada@example.com').should('be.visible')
    cy.contains('Grace Hopper').should('be.visible')
    cy.contains('grace@example.com').should('be.visible')
    cy.contains('No contacts loaded.').should('not.exist')
  })
})
