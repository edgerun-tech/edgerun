// SPDX-License-Identifier: Apache-2.0

describe('intent ui opencode session hydration and switching', () => {
  const seedMessage = (id, role, text, timestamp) => ({
    id,
    role,
    text,
    createdAt: timestamp
  })

  it('hydrates sessions from current+legacy stores and switches to restored messages', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.localStorage.removeItem('intent-ui-opencode-sessions')
        win.localStorage.removeItem('intent-ui-opencode-session-messages')
        win.localStorage.removeItem('intent-ui-codex-sessions')
        win.localStorage.removeItem('intent-ui-codex-session-messages')

        win.localStorage.setItem('intent-ui-opencode-sessions', JSON.stringify([
          {
            sessionId: 'thread-current',
            threadId: 'thread-current',
            provider: 'opencode',
            preview: 'Current session preview',
            updatedAt: '2026-03-01T10:00:00.000Z'
          }
        ]))

        win.localStorage.setItem('intent-ui-codex-sessions', JSON.stringify([
          {
            sessionId: 'thread-legacy',
            threadId: 'thread-legacy',
            provider: 'opencode',
            preview: 'Legacy session preview',
            updatedAt: '2026-02-28T10:00:00.000Z'
          }
        ]))

        win.localStorage.setItem('intent-ui-opencode-session-messages', JSON.stringify({
          'thread-current': [
            seedMessage('m1', 'user', 'Current user question', '2026-03-01T10:00:01.000Z'),
            seedMessage('m2', 'assistant', 'Current assistant reply', '2026-03-01T10:00:02.000Z')
          ],
          'thread-derived-only': [
            seedMessage('m3', 'assistant', 'Derived-only assistant reply', '2026-03-01T09:59:00.000Z')
          ]
        }))

        win.localStorage.setItem('intent-ui-codex-session-messages', JSON.stringify({
          'thread-legacy': [
            seedMessage('m4', 'user', 'Legacy user question', '2026-02-28T10:00:01.000Z'),
            seedMessage('m5', 'assistant', 'Legacy assistant reply', '2026-02-28T10:00:02.000Z')
          ]
        }))
      }
    })

    cy.window().then((win) => {
      const state = win.__intentDebug.getWorkflowUi()
      const sessionIds = (state.sessionHistory || []).map((entry) => entry.sessionId)
      expect(sessionIds).to.include.members(['thread-current', 'thread-legacy', 'thread-derived-only'])
      expect(state.sessionId).to.eq('thread-current')
      expect(String(state.responseText || '')).to.include('Current assistant reply')

      expect(win.__intentDebug.switchSession('thread-legacy')).to.eq(true)
      const switchedLegacy = win.__intentDebug.getWorkflowUi()
      expect(switchedLegacy.sessionId).to.eq('thread-legacy')
      expect((switchedLegacy.messages || []).some((message) => String(message.text || '').includes('Legacy assistant reply'))).to.eq(true)

      expect(win.__intentDebug.switchSession('thread-derived')).to.eq(true)
      const switchedDerived = win.__intentDebug.getWorkflowUi()
      expect(switchedDerived.sessionId).to.eq('thread-derived-only')
      expect((switchedDerived.messages || []).some((message) => String(message.text || '').includes('Derived-only assistant reply'))).to.eq(true)
    })
  })
})
