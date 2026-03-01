// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui codex session persistence and resume', () => {
  const clearRuntimeState = (win) => {
    win.localStorage.removeItem('intent-ui-integrations-v1')
    win.localStorage.removeItem('intent-ui-codex-sessions')
    win.localStorage.removeItem('intent-ui-codex-session-messages')
    win.localStorage.removeItem('qwen_token')
  }

  it('persists sessions, hydrates after reload, and resumes from selected session', () => {
    let callCount = 0
    cy.intercept('POST', '/api/assistant', (req) => {
      callCount += 1
      if (callCount === 1) {
        req.reply({
          statusCode: 200,
          body: {
            ok: true,
            message: 'First session reply.',
            sessionId: 'thread-a',
            threadId: 'thread-a'
          }
        })
        return
      }
      if (callCount === 2) {
        req.reply({
          statusCode: 200,
          body: {
            ok: true,
            message: 'Second session reply.',
            sessionId: 'thread-b',
            threadId: 'thread-b'
          }
        })
        return
      }
      req.reply({
        statusCode: 200,
        body: {
          ok: true,
          message: `Resumed ${req.body?.threadId || 'unknown'}.`,
          sessionId: String(req.body?.threadId || req.body?.sessionId || ''),
          threadId: String(req.body?.threadId || req.body?.sessionId || '')
        }
      })
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

    cy.window().then((win) => win.__intentDebug.askAssistant('first prompt', { provider: 'codex' }))
    cy.wait('@assistantCall').its('request.body').should((body) => {
      expect(body.threadId).to.eq('')
      expect(body.provider).to.eq('codex')
    })

    cy.window().then((win) => win.__intentDebug.askAssistant('second prompt', { provider: 'codex' }))
    cy.wait('@assistantCall').its('request.body').should((body) => {
      expect(body.threadId).to.eq('thread-a')
    })

    cy.window().then((win) => {
      const history = JSON.parse(win.localStorage.getItem('intent-ui-codex-sessions') || '[]')
      expect(history.map((entry) => entry.sessionId)).to.deep.equal(['thread-b', 'thread-a'])
      expect(typeof win.__intentDebug.switchSession).to.eq('function')
      expect(win.__intentDebug.switchSession('2')).to.eq(true)
      const state = win.__intentDebug.getWorkflowUi()
      expect(state.sessionId).to.eq('thread-a')
      expect(state.threadId).to.eq('thread-a')
    })

    cy.window().then((win) => win.__intentDebug.askAssistant('resume first session', { provider: 'codex' }))
    cy.wait('@assistantCall').its('request.body').should((body) => {
      expect(body.threadId).to.eq('thread-a')
      expect(body.sessionId).to.eq('thread-a')
    })

    cy.reload()
    cy.window().then((win) => {
      const state = win.__intentDebug.getWorkflowUi()
      expect((state.sessionHistory || []).map((entry) => entry.sessionId)).to.include.members(['thread-a', 'thread-b'])
      expect(state.sessionId).to.eq('thread-a')
      expect(state.threadId).to.eq('thread-a')
      expect(String(state.responseText || '')).to.include('Resumed thread-a')
    })
  })
})
