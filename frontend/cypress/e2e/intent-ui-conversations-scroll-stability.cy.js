// SPDX-License-Identifier: Apache-2.0

describe('intent ui conversations scroll stability', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_conversations_scroll')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  const seedAiConversationMessages = (win, count = 140) => {
    const now = Date.now()
    const messages = Array.from({ length: count }, (_, index) => ({
      id: `seed-${index + 1}`,
      role: index % 2 === 0 ? 'assistant' : 'user',
      text: `seed message ${index + 1}`,
      createdAt: new Date(now - (count - index) * 1000).toISOString(),
      channel: 'ai',
      author: index % 2 === 0 ? 'Assistant' : 'You'
    }))
    win.localStorage.setItem('intent-ui-local-conversation-messages-v1', JSON.stringify({
      'ai-active': messages
    }))
  }

  it('keeps viewport anchored while loading older messages near top', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.removeItem('intent-ui-opencode-sessions')
        win.localStorage.removeItem('intent-ui-opencode-session-messages')
        win.localStorage.removeItem('intent-ui-codex-sessions')
        win.localStorage.removeItem('intent-ui-codex-session-messages')
        seedAiConversationMessages(win)
      }
    })

    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.contains('Active AI session').click({ force: true })
    cy.contains('Scroll up to load older messages (80/140)').should('be.visible')

    cy.get('[data-testid="conversation-thread-scroll"]').then(($container) => {
      const container = $container[0]
      container.scrollTop = 0
      container.dispatchEvent(new Event('scroll', { bubbles: true }))
    })

    cy.contains('Scroll up to load older messages (80/140)').should('not.exist')
    cy.get('[data-testid="conversation-thread-scroll"]').should(($container) => {
      const container = $container[0]
      expect(container.scrollTop).to.be.greaterThan(0)
    })
  })
})
