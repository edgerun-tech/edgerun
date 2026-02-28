// SPDX-License-Identifier: Apache-2.0

describe('intent ui conversations hub', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_conversations_hub')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows empty-state guidance and provider status in conversation settings', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('demo-emails')
        win.localStorage.removeItem('demo-email-index-v1')
        win.localStorage.removeItem('google_token')
        win.localStorage.removeItem('whatsapp_token')
        win.localStorage.removeItem('messenger_token')
        win.localStorage.removeItem('telegram_token')
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')
    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.get('[data-testid="drawer-suggestions-right-conversations"]').should('be.visible')
    cy.get('[data-testid="drawer-suggestions-list-right-conversations"]').should('be.visible')
    cy.get('[data-testid="drawer-suggestion-conversations-email"]').should('be.visible')

    cy.get('[data-testid="conversations-empty-state"]').should('contain.text', 'This is where all your conversations will be available')
    cy.get('[data-testid="conversation-provider-email"]').should('be.visible')
    cy.get('[data-testid="conversation-provider-whatsapp"]').should('be.visible')
    cy.get('[data-testid="conversation-provider-messenger"]').should('be.visible')
    cy.get('[data-testid="conversation-provider-telegram"]').should('be.visible')

    cy.contains('Active AI session').click({ force: true })
    cy.get('[data-testid="conversation-settings-toggle"]').click({ force: true })
    cy.get('[data-testid="conversation-settings-popup"]').should('be.visible')
    cy.get('[data-testid="conversation-settings-provider-email"]').should('contain.text', 'Email')

    cy.get('[data-testid="conversation-emoji-toggle"]').click({ force: true })
    cy.contains('button', '😀').click({ force: true })
    cy.get('[data-testid="conversation-draft-input"]').type('hello from conversations hub', { force: true })
    cy.get('[data-testid="conversation-send-message"]').click({ force: true })

    cy.contains('hello from conversations hub').should('exist')
  })
})
