// SPDX-License-Identifier: Apache-2.0

describe('intent ui conversations recency and filters', () => {
  it('sorts by recency and applies source+search filters with persistence and shortcut', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
        win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_conversation_recency_filter')
        win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
        win.sessionStorage.setItem(
          'intent-ui-profile-scopes-v1',
          JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
        )

        win.localStorage.setItem('intent-ui-local-conversation-messages-v1', JSON.stringify({
          'call-link-older': [{
            id: 'old-1',
            role: 'assistant',
            text: 'Pending call link copied: https://example.com/older',
            createdAt: '2026-03-01T08:00:00.000Z',
            channel: 'call',
            author: 'Call Studio',
            threadTitle: 'Older Call Thread',
            threadSubtitle: 'Awaiting recipient',
            callStatus: 'pending'
          }],
          'call-link-recent': [{
            id: 'new-1',
            role: 'assistant',
            text: 'Pending call link copied: https://example.com/recent',
            createdAt: '2026-03-01T09:30:00.000Z',
            channel: 'call',
            author: 'Call Studio',
            threadTitle: 'Recent Call Thread',
            threadSubtitle: 'Awaiting recipient',
            callStatus: 'pending'
          }]
        }))

        class FakeWebSocket {
          static CONNECTING = 0
          static OPEN = 1
          static CLOSED = 3

          constructor() {
            this.readyState = FakeWebSocket.CONNECTING
            this.binaryType = 'arraybuffer'
            setTimeout(() => {
              this.readyState = FakeWebSocket.OPEN
              if (typeof this.onopen === 'function') this.onopen(new Event('open'))
            }, 0)
          }

          send() {}
          close() {
            this.readyState = FakeWebSocket.CLOSED
            if (typeof this.onclose === 'function') this.onclose(new Event('close'))
          }
        }
        win.WebSocket = FakeWebSocket

        const nativeFetch = win.fetch.bind(win)
        win.fetch = (input, init) => {
          const requestUrl = typeof input === 'string' ? input : String(input?.url || '')
          if (requestUrl.includes('/v1/local/node/info.pb')) {
            return Promise.resolve(new win.Response(new Uint8Array([8, 1]), {
              status: 200,
              headers: { 'content-type': 'application/octet-stream' }
            }))
          }
          return nativeFetch(input, init)
        }
      }
    })

    cy.get('button[title="Conversations"]').first().click({ force: true })

    cy.get('[data-testid="conversation-thread-item"]').eq(0).should('contain.text', 'Recent Call Thread')
    cy.get('[data-testid="conversation-thread-item"]').eq(1).should('contain.text', 'Older Call Thread')

    cy.get('[data-testid="conversation-thread-source-option-call"]').click({ force: true })
    cy.get('[data-testid="conversation-thread-item"]').each(($row) => {
      expect($row.attr('data-conversation-channel')).to.eq('call')
    })
    cy.get('[data-testid="conversation-thread-item"]').eq(0).should('contain.text', 'Recent Call Thread')

    cy.get('[data-testid="conversation-thread-search-input"]').clear().type('older')
    cy.get('[data-testid="conversation-thread-item"]').should('have.length', 1)
    cy.get('[data-testid="conversation-thread-item"]').eq(0).should('contain.text', 'Older Call Thread')

    cy.reload()
    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.get('[data-testid="conversation-thread-search-input"]').should('have.value', 'older')
    cy.get('[data-testid="conversation-thread-item"]').should('have.length', 1)
    cy.get('[data-testid="conversation-thread-item"]').eq(0).should('contain.text', 'Older Call Thread')

    cy.get('body').click(0, 0)
    cy.get('body').type('/')
    cy.focused().should('have.attr', 'data-testid', 'conversation-thread-search-input')

    cy.get('[data-testid="conversation-thread-search-clear"]').click({ force: true })
    cy.get('[data-testid="conversation-thread-item"]').eq(0).should('contain.text', 'Recent Call Thread')
  })
})
