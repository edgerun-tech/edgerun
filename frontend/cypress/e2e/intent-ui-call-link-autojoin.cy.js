// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui call link autojoin', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_call_autojoin')
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

      answer() {}

      close() {
        const handler = this.handlers.close
        if (typeof handler === 'function') handler()
      }

      emit(event, payload) {
        const handler = this.handlers[event]
        if (typeof handler === 'function') handler(payload)
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
        const call = new FakeCall(target)
        setTimeout(() => {
          call.emit('stream', { id: 'remote-stream' })
        }, 40)
        return call
      }

      connect(target) {
        const conn = new FakeConn(target)
        setTimeout(() => {
          const open = conn.handlers.open
          if (typeof open === 'function') open()
        }, 10)
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

  it('opens call window and auto-connects when visiting shared link', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        seedProfileSession(win)
        installFakePeerAndMedia(win)
        win.history.replaceState({}, '', '/call/remote-peer-777')
      }
    })

    cy.contains('Call Studio').should('be.visible')
    cy.get('input[placeholder="Enter call ID to join"]').should('have.value', 'remote-peer-777')
    cy.contains(/Joining call from shared link|Calling\.{3}|Connected!|1 peer connected/).should('be.visible')
    cy.contains('Copy Link').should('exist')
    cy.contains('New Call ID').should('not.exist')
  })
})
