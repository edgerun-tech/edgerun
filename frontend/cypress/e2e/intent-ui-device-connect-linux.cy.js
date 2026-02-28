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
    cy.intercept('POST', '/api/tunnel/create-pairing-code', {
      statusCode: 200,
      body: {
        ok: true,
        error: '',
        pairingCode: 'AUTO-PAIR-0001',
        expiresUnixMs: 1767225600000,
        deviceCommand: 'edgerun-node-manager tunnel-connect --pairing-code AUTO-PAIR-0001'
      }
    }).as('issuePairing')

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

    cy.get('[data-testid="device-domain-input"]').clear().type('alice.users.edgerun.tech')
    cy.get('[data-testid="device-registration-token-input"]').clear().type('reg_tok_123')
    cy.get('[data-testid="device-issue-pairing-code"]').click()
    cy.wait('@issuePairing')
    cy.get('[data-testid="device-pairing-status"]').should('contain.text', 'Pairing code issued')
    cy.get('[data-testid="device-pairing-code-input"]').should('have.value', 'AUTO-PAIR-0001')
    cy.get('[data-testid="device-linux-script"]').should('contain.text', 'AUTO-PAIR-0001')

    cy.get('[data-testid="device-pairing-code-input"]').clear().type('CCCCC-BRCF-DICT-EINT')
    cy.get('[data-testid="device-linux-script"]').should('contain.text', 'CCCCC-BRCF-DICT-EINT')

    cy.get('[data-testid="device-copy-script"]').should('be.visible')
  })
})
