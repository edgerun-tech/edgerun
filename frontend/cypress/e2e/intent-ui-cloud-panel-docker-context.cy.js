// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui cloud panel docker context actions', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_cloud_docker_context')
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

  it('opens right-click menu on docker container and updates state', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        seedProfileSession(win)
      }
    })

    cy.window().then((win) => {
      win.__intentDebug.openWindow('cloud')
    })

    cy.contains('Cloud').should('exist')
    cy.contains('edgerun-dev-api').should('exist')

    cy.get('[data-testid="cloud-resource-status-docker-ctr-ctr-sim-1"]')
      .trigger('contextmenu', { force: true })

    cy.get('[data-testid="intent-context-menu"]').should('exist')
    cy.get('[data-testid="intent-context-action-stop-container"]').click({ force: true })

    cy.get('[data-testid="cloud-resource-status-docker-ctr-ctr-sim-1"]').should('contain.text', 'exited')

    cy.get('[data-testid="cloud-resource-status-docker-ctr-ctr-sim-1"]')
      .trigger('contextmenu', { force: true })
    cy.get('[data-testid="intent-context-action-start-container"]').click({ force: true })
    cy.get('[data-testid="cloud-resource-status-docker-ctr-ctr-sim-1"]').should('contain.text', 'running')
  })
})
