describe('intent ui local eventbus loopback', () => {
  it('opens settings from IntentBar quick action without bridge websocket', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
        win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_loopback_test')
        win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
        win.sessionStorage.setItem(
          'intent-ui-profile-scopes-v1',
          JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
        )

        class FakeWebSocket {
          static OPEN = 1
          static CLOSED = 3

          constructor() {
            this.readyState = FakeWebSocket.CLOSED
            setTimeout(() => {
              if (typeof this.onerror === 'function') this.onerror(new Event('error'))
              if (typeof this.onclose === 'function') this.onclose()
            }, 0)
          }

          send() {
            throw new Error('websocket unavailable in test')
          }

          close() {
            this.readyState = FakeWebSocket.CLOSED
            if (typeof this.onclose === 'function') this.onclose()
          }
        }

        win.WebSocket = FakeWebSocket
        win.localStorage.setItem('intent-ui-intentbar-pinned-v1', '1')
      }
    })

    cy.get('button[title="Settings"]').first().click({ force: true })
    cy.get('[aria-label="Settings window"]').should('exist')
  })
})
