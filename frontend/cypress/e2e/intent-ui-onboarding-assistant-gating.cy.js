// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

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

  it('allows codex assistant execution with linked codex_cli outside profile mode', () => {
    cy.intercept('POST', '/api/assistant', {
      statusCode: 200,
      body: {
        ok: true,
        message: 'Assistant response outside profile mode.'
      }
    }).as('assistantCall')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        clearRuntimeState(win)
        win.localStorage.setItem('intent-ui-integrations-v1', JSON.stringify({
          codex_cli: {
            connected: true,
            linked: true,
            connectorMode: 'user_owned',
            accountLabel: 'Codex CLI Session',
            capabilities: ['assistant.local_cli.execute']
          }
        }))
      }
    })

    cy.window().then((win) => {
      expect(typeof win.__intentDebug?.askAssistant).to.eq('function')
      win.__intentDebug.askAssistant('run codex without profile', { provider: 'codex' })
    })

    cy.wait('@assistantCall').its('request.body').should((body) => {
      expect(body.provider).to.eq('codex')
      expect(String(body.message || '')).to.include('run codex without profile')
    })

    cy.window().its('__intentDebug').should((debugApi) => {
      const state = debugApi.getWorkflowUi()
      expect(state.codexPhase).to.eq('done')
      expect(String(state.responseText || '')).to.include('outside profile mode')
      const blocked = state.statusEvents.some((event) =>
        String(event?.detail || '').includes('connect Codex CLI integration first')
      )
      expect(blocked).to.eq(false)
    })
  })
})
