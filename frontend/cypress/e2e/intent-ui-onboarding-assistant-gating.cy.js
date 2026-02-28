// SPDX-License-Identifier: Apache-2.0

describe('intent ui onboarding and assistant integration gating', () => {

  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_onboarding_gate')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  const clearRuntimeState = (win) => {
    win.localStorage.removeItem('intent-ui-integrations-v1')
    win.localStorage.removeItem('qwen_token')
    win.localStorage.removeItem('intent-ui-profile-blob-browser-v1')
    win.localStorage.removeItem('intent-ui-profile-blob-google-v1')
    win.localStorage.removeItem('intent-ui-profile-blob-git-v1')
    win.localStorage.removeItem('intent-ui-profile-sync-pending-v1')
    win.sessionStorage.removeItem('intent-ui-profile-mode-v1')
    win.sessionStorage.removeItem('intent-ui-profile-id-v1')
    win.sessionStorage.removeItem('intent-ui-profile-backend-v1')
    win.sessionStorage.removeItem('intent-ui-profile-scopes-v1')
  }

  it('keeps onboarding reachable and blocks assistant until integration is connected', () => {
    cy.intercept('POST', '/api/assistant').as('assistantCall')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        clearRuntimeState(win)
        seedProfileSession(win)
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.get('[data-testid="account-circle-trigger"]').click({ force: true })
    cy.get('[data-testid="open-profile-bootstrap-gate"]').click({ force: true })
    cy.get('[data-testid="profile-bootstrap-gate"]').should('be.visible')
    cy.get('[data-testid="profile-bootstrap-dismiss"]').click({ force: true })
    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.window().then((win) => {
      expect(typeof win.__intentDebug?.openWindow).to.eq('function')
      win.__intentDebug.openWindow('guide')
    })

    cy.contains('Startup Tasks').should('be.visible')
    cy.contains('Assistant integration').should('be.visible')

    cy.window().then((win) => {
      expect(typeof win.__intentDebug?.askAssistant).to.eq('function')
      win.__intentDebug.askAssistant('test assistant gate', { provider: 'codex' })
      const state = win.__intentDebug.getWorkflowUi()
      expect(state.codexPhase).to.eq('error')
      const blocked = state.statusEvents.some((event) =>
        String(event?.detail || '').includes('connect Codex CLI integration first')
      )
      expect(blocked).to.eq(true)
    })

    cy.get('@assistantCall.all').should('have.length', 0)
  })
})
