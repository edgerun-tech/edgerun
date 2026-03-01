// SPDX-License-Identifier: Apache-2.0

import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui settings widgets unified', () => {
  const clearProfileState = (win) => {
    win.localStorage.removeItem('intent-ui-profile-blob-browser-v1')
    win.localStorage.removeItem('intent-ui-profile-blob-google-v1')
    win.localStorage.removeItem('intent-ui-profile-blob-git-v1')
    win.localStorage.removeItem('intent-ui-profile-sync-pending-v1')
    win.sessionStorage.removeItem('intent-ui-profile-mode-v1')
    win.sessionStorage.removeItem('intent-ui-profile-id-v1')
    win.sessionStorage.removeItem('intent-ui-profile-backend-v1')
    win.sessionStorage.removeItem('intent-ui-profile-scopes-v1')
  }

  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'settings_widgets_test')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows widget controls inside settings and no separate widgets CTA', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        clearProfileState(win)
        seedProfileSession(win)
        installLocalBridgeSimulator(win)
      }
    })

    cy.contains('Local Bridge Required').should('not.exist')

    cy.window().should((win) => {
      expect(win.__intentDebug?.getEventBusRuntime).to.be.a('function')
      expect(win.__intentDebug.getEventBusRuntime().localBridgeConnected).to.eq(true)
    })

    cy.window().then((win) => {
      expect(win.__intentDebug?.openWindow).to.be.a('function')
      win.__intentDebug.openWindow('settings')
    })
    cy.get('[data-testid="settings-panel"]', { timeout: 10000 }).should('be.visible')

    cy.get('[data-testid="settings-widgets-section"]').should('be.visible')
    cy.get('[data-testid="settings-widget-toggle-map"]').check({ force: true }).should('be.checked')
    cy.get('[data-testid="settings-widget-toggle-map"]').uncheck({ force: true }).should('not.be.checked')

    cy.contains('button', 'Open Widgets').should('not.exist')
  })
})
