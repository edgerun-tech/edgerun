// SPDX-License-Identifier: Apache-2.0

describe('intent ui linux first device connect', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_device_connect')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('shows linux platform script and updates it with pairing code', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Devices panel"]').first().click({ force: true })

    cy.get('[data-testid="device-connect-block"]').should('be.visible')
    cy.get('[data-testid="device-platform-linux"]').should('be.visible')
    cy.get('[data-testid="device-platform-macos"]').should('be.disabled')
    cy.get('[data-testid="device-platform-windows"]').should('be.disabled')

    cy.get('[data-testid="device-linux-script"]').should('contain.text', '--local-bridge-listen 127.0.0.1:7777')
    cy.get('[data-testid="device-linux-script"]').should('contain.text', 'install-node-manager.sh | sh -s -- --bridge-listen 127.0.0.1:7777')
    cy.get('[data-testid="device-linux-script"]').should('contain.text', '<PAIRING_CODE>')

    cy.get('[data-testid="device-pairing-code-input"]').clear().type('CCCCC-BRCF-DICT-EINT')
    cy.get('[data-testid="device-linux-script"]').should('contain.text', 'CCCCC-BRCF-DICT-EINT')

    cy.get('[data-testid="device-copy-script"]').should('be.visible')
  })
})
