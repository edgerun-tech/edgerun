// SPDX-License-Identifier: Apache-2.0

describe('intent ui workflow modal gating', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_workflow_modal_gating')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('does not open code-edit modal when integrations drawer is opened', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.removeItem('intent-ui-opencode-sessions')
        win.localStorage.removeItem('intent-ui-opencode-session-messages')
        win.localStorage.removeItem('intent-ui-codex-sessions')
        win.localStorage.removeItem('intent-ui-codex-session-messages')
      }
    })

    cy.get('button[title="Conversations"]').first().click({ force: true })
    cy.get('[data-testid="drawer-suggestion-conversations-email"]').click({ force: true })

    cy.window().then((win) => {
      const workflow = win.__intentDebug?.getWorkflowUi?.()
      expect(workflow?.leftOpen).to.equal(true)
      expect(workflow?.leftPanel).to.equal('integrations')
    })
    cy.contains('h3', 'Code Edit Workflow').should('not.exist')
  })
})
