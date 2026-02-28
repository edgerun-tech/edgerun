// SPDX-License-Identifier: Apache-2.0

describe('intent ui profile bootstrap gate', () => {
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
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_bootstrap_test')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('requires create/load profile and does not expose ephemeral mode', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        clearProfileState(win)
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('be.visible')
    cy.get('[data-testid="profile-bootstrap-tab-create"]').should('contain.text', 'Create profile')
    cy.get('[data-testid="profile-bootstrap-tab-load"]').should('contain.text', 'Load profile')
    cy.get('[data-testid="profile-bootstrap-ephemeral"]').should('not.exist')
    cy.get('[data-testid="profile-bootstrap-submit"]').should('contain.text', 'Create encrypted profile')
  })

  it('allows reopening/dismissing gate when profile is loaded and locks again after reset', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        clearProfileState(win)
        seedProfileSession(win)
      }
    })

    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.get('[data-testid="account-circle-trigger"]').click({ force: true })
    cy.get('[data-testid="open-profile-bootstrap-gate"]').click({ force: true })
    cy.get('[data-testid="profile-bootstrap-gate"]').should('be.visible')
    cy.get('[data-testid="profile-bootstrap-dismiss"]').click({ force: true })
    cy.get('[data-testid="profile-bootstrap-gate"]').should('not.exist')

    cy.get('[data-testid="account-circle-trigger"]').click({ force: true })
    cy.get('[data-testid="account-reset-session"]').click({ force: true })
    cy.get('[data-testid="profile-bootstrap-gate"]').should('be.visible')
    cy.get('[data-testid="profile-bootstrap-dismiss"]').should('not.exist')
  })
})
