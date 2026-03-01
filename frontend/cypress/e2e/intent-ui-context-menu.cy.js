// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui right click context menu', () => {
  it('opens and closes the global context menu', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
      }
    })

    cy.get('[data-input-layer]').rightclick('center')
    cy.get('[data-testid="intent-context-menu"]').should('be.visible')
    cy.get('[data-testid="intent-context-action-new-assistant-session"]').should('exist')

    cy.get('body').click(8, 8)
    cy.get('[data-testid="intent-context-menu"]').should('not.exist')
  })
})
