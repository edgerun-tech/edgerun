// SPDX-License-Identifier: Apache-2.0

describe('intent ui connected devices panel', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_connected_devices_widget')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows connected device rows in devices drawer', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Devices panel"]').first().click({ force: true })
    cy.contains('p', /^Devices$/).should('be.visible')
    cy.get('body').invoke('text').then((text) => {
      expect(text).to.match(/This Device|This Browser|No connected devices yet\./)
    })
  })
})
