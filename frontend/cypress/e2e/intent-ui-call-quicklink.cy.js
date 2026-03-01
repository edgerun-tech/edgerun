// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui call quicklink', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_call_quicklink')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  const installFakePeerAndMedia = (win) => {
    class FakeConn {
      constructor(peer) {
        this.peer = peer
        this.handlers = {}
      }

      on(event, handler) {
        this.handlers[event] = handler
      }

      send() {}

      close() {
        const handler = this.handlers.close
        if (typeof handler === 'function') handler()
      }
    }

    class FakeCall {
      constructor(peer) {
        this.peer = peer
        this.handlers = {}
      }

      on(event, handler) {
        this.handlers[event] = handler
      }

      close() {
        const handler = this.handlers.close
        if (typeof handler === 'function') handler()
      }
    }

    class FakePeer {
      constructor() {
        this.handlers = {}
        setTimeout(() => {
          const open = this.handlers.open
          if (typeof open === 'function') open('local-peer-123')
        }, 0)
      }

      on(event, handler) {
        this.handlers[event] = handler
      }

      call(target) {
        return new FakeCall(target)
      }

      connect(target) {
        const conn = new FakeConn(target)
        setTimeout(() => {
          const open = conn.handlers.open
          if (typeof open === 'function') open()
        }, 5)
        return conn
      }

      destroy() {}
    }

    win.__intentUiPeerFactory = () => new FakePeer()

    const mediaDevices = win.navigator.mediaDevices || {}
    if (!win.navigator.mediaDevices) {
      Object.defineProperty(win.navigator, 'mediaDevices', {
        value: mediaDevices,
        configurable: true
      })
    }
    mediaDevices.getUserMedia = () => {
      const track = { enabled: true, stop() {} }
      return Promise.resolve({
        getTracks: () => [track],
        getVideoTracks: () => [track],
        getAudioTracks: () => [track]
      })
    }
  }

  it('opens call studio, copies link, and persists pending thread', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        seedProfileSession(win)
        installFakePeerAndMedia(win)
        win.localStorage.removeItem('intent-ui-local-conversation-messages-v1')
        const clipboard = win.navigator.clipboard || {}
        if (!win.navigator.clipboard) {
          Object.defineProperty(win.navigator, 'clipboard', {
            value: clipboard,
            configurable: true
          })
        }
        clipboard.writeText = cy.stub().as('clipboardWrite')
      }
    })

    cy.get('[data-testid="intentbar-quick-call"]').click({ force: true })
    cy.contains('Call Studio').should('be.visible')

    cy.get('@clipboardWrite').should('have.been.called')
    cy.get('@clipboardWrite').should('have.been.calledWithMatch', /\/call\/local-peer-123$/)

    cy.window().then((win) => {
      const raw = win.localStorage.getItem('intent-ui-local-conversation-messages-v1') || '{}'
      const parsed = JSON.parse(raw)
      expect(parsed['call-link-local-peer-123']).to.be.an('array').and.have.length.greaterThan(0)
      expect(parsed['call-link-local-peer-123'][0].text).to.contain('Pending call link copied:')
    })
  })
})
