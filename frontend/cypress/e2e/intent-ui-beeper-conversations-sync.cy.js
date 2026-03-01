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
            participants: {
              items: [
                { id: '@other:beeper.local', fullName: 'Beeper Bot', imgURL: 'https://example.com/avatar.jpg', isSelf: false },
                { id: '@self:beeper.com', fullName: 'You', isSelf: true }
              ]
            },
            lastActivity: '2026-03-01T06:10:00.000Z',
            preview: {
              type: 'TEXT',
              senderName: 'Beeper Bot',
              text: 'Welcome to Beeper Desktop API sync.'
            }
          }
        ]
      }
    }).as('beeperChats')

    cy.intercept('GET', '/api/beeper/messages*', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          {
            id: 'm1',
            senderName: 'Beeper Bot',
            timestamp: '2026-03-01T06:09:00.000Z',
            text: 'Welcome to Beeper Desktop API sync.',
            isSender: false,
            attachments: []
          },
          {
            id: 'm2',
            senderName: 'Beeper Bot',
            timestamp: '2026-03-01T06:10:00.000Z',
            type: 'IMAGE',
            isSender: false,
            attachments: [
              { type: 'image', srcURL: 'https://example.com/photo.jpg' }
            ]
          }
        ]
      }
    }).as('beeperMessages')

    cy.intercept('GET', '/api/beeper/imported*', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          {
            id: 'bridge-beeper-import-old-thread',
            title: 'Imported Legacy Chat',
            subtitle: 'Imported from Facebook export',
            preview: 'Imported message preview',
            updatedAt: '2025-12-01T00:00:00.000Z',
            messages: [
              {
                id: 'legacy-1',
                role: 'contact',
                author: 'Legacy Contact',
                channel: 'beeper',
                text: 'Imported message preview',
                createdAt: '2025-12-01T00:00:00.000Z'
              }
            ]
          }
        ]
      }
    }).as('beeperImported')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.removeItem('intent-ui-local-conversation-messages-v1')
        win.localStorage.setItem('beeper_access_token', 'beeper_access_token_seeded_for_sync')
      }
    })

    cy.wait('@beeperChats')
    cy.wait('@beeperMessages')
    cy.wait('@beeperImported')
    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.contains('Beeper Team Chat').should('be.visible')
    cy.contains('Beeper Bot: [Photo]').should('be.visible')
    cy.contains('Imported Legacy Chat').should('be.visible')
    cy.contains('Beeper Team Chat').click({ force: true })
    cy.contains('Welcome to Beeper Desktop API sync.').should('be.visible')
    cy.contains('https://example.com/photo.jpg').should('be.visible')
  })
})
