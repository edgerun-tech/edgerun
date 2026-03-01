// SPDX-License-Identifier: Apache-2.0

describe('intent ui connected devices widget', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_connected_devices_widget')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows centered connected device rows with IP and flag info', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
      }
    })

    cy.get('[data-testid="connected-devices-widget"]').should('be.visible')
    cy.get('[data-testid="connected-devices-widget-row"]').should('have.length.at.least', 1)
    cy.contains('This Browser').should('be.visible')
    cy.contains('--').should('be.visible')
  })
})
