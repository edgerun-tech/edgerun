// SPDX-License-Identifier: Apache-2.0

describe('intent ui conversations chat bubbles', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_chat_bubbles')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('opens and moves floating chat bubble from thread right-click', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.removeItem('intent-ui-local-conversation-messages-v1')
        win.localStorage.removeItem('intent-ui-chat-bubbles-v1')
      }
    })

    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.contains('Active AI session').click({ force: true })
    cy.get('[data-testid="conversation-draft-input"]').type('chat bubble seed message', { force: true })
    cy.get('[data-testid="conversation-send-message"]').click({ force: true })

    cy.get('[data-testid="conversation-thread-message"]').last().trigger('contextmenu', { button: 2, force: true })
    cy.get('[data-testid="conversation-chat-bubble"]').should('exist')
    cy.contains('[data-testid="conversation-chat-bubble"]', 'chat bubble seed message').should('be.visible')

    cy.get('[data-testid="conversation-chat-bubble"]').first().then(($bubble) => {
      const beforeLeft = parseFloat($bubble[0].style.left || '0')
      const beforeTop = parseFloat($bubble[0].style.top || '0')
      cy.get('[data-testid="conversation-chat-bubble-drag"]').first()
        .trigger('pointerdown', { button: 0, clientX: beforeLeft + 40, clientY: beforeTop + 16, force: true })
      cy.get('body').trigger('pointermove', { clientX: beforeLeft + 120, clientY: beforeTop + 90, force: true })
      cy.get('body').trigger('pointerup', { force: true })
      cy.get('[data-testid="conversation-chat-bubble"]').first().should(($next) => {
        const afterLeft = parseFloat($next[0].style.left || '0')
        const afterTop = parseFloat($next[0].style.top || '0')
        expect(afterLeft).to.not.equal(beforeLeft)
        expect(afterTop).to.not.equal(beforeTop)
      })
    })
  })
})
