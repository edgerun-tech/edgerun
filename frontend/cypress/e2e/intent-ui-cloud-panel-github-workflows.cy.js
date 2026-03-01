// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui cloud panel github workflows', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_cloud_github_workflows')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify([
        'openid',
        'profile',
        'edgerun:profile.read',
        'edgerun:profile.write',
        'edgerun:intents.submit',
        'edgerun:cap.network.use'
      ])
    )
  }

  it('shows remote workflow runs and appends local runner run', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.setItem('github_token', 'ghp_test_token_cloud_panel_workflows_12345')
        win.localStorage.setItem('intent-ui-integrations-v1', JSON.stringify({
          github: {
            connected: true,
            linked: true,
            connectorMode: 'user_owned',
            authMethod: 'token',
            capabilities: ['repos.read', 'repos.write', 'prs.read', 'prs.write'],
            connectedAt: new Date().toISOString(),
            accountLabel: 'GitHub Account'
          }
        }))
        seedProfileSession(win)
      }
    })

    cy.window().then((win) => {
      win.__intentDebug.openWindow('cloud')
    })

    cy.contains('Cloud').should('exist')
    cy.contains('ken/edgerun · ci').should('exist')

    cy.get('[data-testid="cloud-panel-run-local-ci"]').click({ force: true })
    cy.contains('local · intent-ui-ci').should('exist')
  })
})
