describe('intent ui terminal term-web', () => {
  it('renders a term-web iframe target instead of mock terminal output', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
        win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_terminal_termweb')
        win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
        win.sessionStorage.setItem(
          'intent-ui-profile-scopes-v1',
          JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
        )
        win.localStorage.setItem('intent-ui-intentbar-pinned-v1', '1')

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
            const bytes = new Uint8Array([8, 1])
            return Promise.resolve(new win.Response(bytes, {
              status: 200,
              headers: { 'content-type': 'application/octet-stream' }
            }))
          }
          return nativeFetch(input, init)
        }
      }
    })

    cy.get('input[placeholder*="What do you want to do?"]').first().type('$ pwd{enter}', { force: true })
    cy.get('[aria-label="Terminal window"]').should('exist')

    cy.get('[data-testid="intent-ui-terminal-target-input"]').clear().type('http://127.0.0.1:4173')
    cy.get('[data-testid="intent-ui-terminal-connect"]').click({ force: true })

    cy.get('[data-testid="intent-ui-terminal-iframe"]', { timeout: 10000 })
      .should('exist')
      .should('have.attr', 'src')
      .and('include', 'http://127.0.0.1:4173/term?sid=')

    cy.get('[data-testid="intent-ui-terminal-ready-state"]', { timeout: 10000 })
      .invoke('text')
      .should((value) => {
        const normalized = String(value || '').trim()
        expect(['ready', 'loading-shell']).to.include(normalized)
      })

    cy.get('input[placeholder*="What do you want to do?"]').first().type('$ whoami{enter}', { force: true })

    cy.get('[data-testid="intent-ui-forwarded-commands"]').within(() => {
      cy.contains('[data-testid="intent-ui-forwarded-command"]', '$ whoami').should('contain.text', 'sent')
    })
  })
})
