// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui onvif widget', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_onvif_widget')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify([
        'openid',
        'profile',
        'edgerun:profile.read',
        'edgerun:profile.write',
        'edgerun:cap.network.use',
        'edgerun:cap.camera.use'
      ])
    )
  }

  it('opens ONVIF panel, scans LAN candidates, and adds a camera card', () => {
    cy.intercept('GET', '**/v1/local/onvif/discover', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          { name: 'Front Gate', ip: '192.168.1.22', url: 'http://192.168.1.22/onvif/device_service' },
          { name: 'Garage', ip: '192.168.1.23', url: 'http://192.168.1.23/onvif/device_service' }
        ]
      }
    }).as('onvifDiscover')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-onvif-cameras-v1')
        seedProfileSession(win)
      }
    })

    cy.get('button[title="Settings"]').first().click({ force: true })
    cy.get('[data-testid="settings-panel"]').should('be.visible')
    cy.get('[data-testid="settings-open-onvif"]').click({ force: true })

    cy.get('[data-testid="onvif-panel"]').should('exist')
    cy.get('[data-testid="onvif-scan-lan"]').click({ force: true })
    cy.wait('@onvifDiscover')
    cy.get('[data-testid="onvif-status"]').should('contain.text', 'Found 2 ONVIF candidates.')
    cy.get('[data-testid="onvif-scan-results"]').should('exist')
    cy.get('[data-testid="onvif-scan-add"]').first().click({ force: true })

    cy.get('[data-testid="onvif-camera-card"]').should('have.length', 1)
    cy.contains('Front Gate').should('exist')
    cy.contains('rtsp://192.168.1.22/stream1').should('exist')
  })
})
