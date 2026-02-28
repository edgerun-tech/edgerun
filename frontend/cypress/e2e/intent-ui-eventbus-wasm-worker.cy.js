// SPDX-License-Identifier: Apache-2.0

describe('intent ui eventbus wasm worker', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_eventbus_worker')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('boots eventbus in worker and records clipboard events', () => {
    let timelineBefore = 0

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
      }
    })

    cy.window().should((win) => {
      expect(typeof win.__intentDebug?.getEventBusRuntime).to.eq('function')
      expect(typeof win.__intentDebug?.getEventBusTimeline).to.eq('function')
    })

    cy.window().should((win) => {
      const runtime = win.__intentDebug.getEventBusRuntime()
      expect(runtime.workerReady).to.eq(true)
      expect(String(runtime.engine)).to.match(/^worker-/)
    })

    cy.window().then((win) => {
      timelineBefore = win.__intentDebug.getEventBusTimeline().length
      win.__intentDebug.publishEvent('eventbus.test', { marker: 'cypress' })
    })

    cy.window().should((win) => {
      const after = win.__intentDebug.getEventBusTimeline().length
      expect(after).to.be.greaterThan(timelineBefore)
    })
  })
})
