// SPDX-License-Identifier: Apache-2.0

describe('intent ui beeper conversations sync', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_beeper_conversations_sync')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('loads Beeper chats through backend conversations source flow', () => {
    cy.intercept('GET', '/api/beeper/chats*', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          {
            id: '!abc123:beeper.local',
            accountID: 'facebookgo',
            network: 'Facebook/Messenger',
            title: 'Beeper Team Chat',
            lastActivity: '2026-03-01T06:10:00.000Z',
            preview: {
              type: 'TEXT',
              senderName: 'Beeper Bot'
            }
          }
        ]
      }
    }).as('beeperChats')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.removeItem('intent-ui-local-conversation-messages-v1')
        win.localStorage.setItem('beeper_access_token', 'beeper_access_token_seeded_for_sync')
      }
    })

    cy.wait('@beeperChats')
    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.contains('Beeper Team Chat').should('be.visible')
  })
})
