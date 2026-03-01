// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui super+v composer launcher', () => {
  const clearRuntimeState = (win) => {
    win.localStorage.removeItem('intent-ui-opencode-sessions')
    win.localStorage.removeItem('intent-ui-opencode-session-messages')
    win.localStorage.removeItem('intent-ui-codex-sessions')
    win.localStorage.removeItem('intent-ui-codex-session-messages')
    win.localStorage.removeItem('intent-ui-local-conversation-messages-v1')
    win.localStorage.removeItem('intent-ui-chat-head-prefs-v1')
  }

  it('opens conversations composer with emoji palette', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        clearRuntimeState(win)
      }
    })

    cy.get('[data-testid="conversation-draft-input"]').should('not.exist')

    cy.get('body').trigger('keydown', {
      key: 'v',
      code: 'KeyV',
      metaKey: true,
      bubbles: true,
      cancelable: true
    })

    cy.get('[data-testid="conversation-draft-input"]').should('be.visible').and('be.focused')
    cy.get('[data-testid="conversation-emoji-palette"]').should('be.visible')
    cy.get('[data-testid="conversation-clipboard-insert"]').should('be.visible')
  })
})
