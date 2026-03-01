// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui daly bms web bluetooth integration', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_daly_test')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('selects daly bms via web bluetooth and completes verify + probe', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('daly_bms_device_id')
        win.localStorage.removeItem('daly_bms_device_name')

        const DALY_NUS_SERVICE_UUID = '6e400001-b5a3-f393-e0a9-e50e24dcca9e'
        const DALY_NUS_TX_UUID = '6e400003-b5a3-f393-e0a9-e50e24dcca9e'
        const DALY_NUS_RX_UUID = '6e400002-b5a3-f393-e0a9-e50e24dcca9e'

        const toBytes = (input) => {
          if (input instanceof Uint8Array) return input
          if (Array.isArray(input)) return Uint8Array.from(input)
          if (typeof input === 'string') return new win.TextEncoder().encode(input)
          return new Uint8Array()
        }
        const asDataView = (input) => {
          const bytes = toBytes(input)
          return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
        }

        const batteryCharacteristic = {
          readValue: () => Promise.resolve(asDataView([81]))
        }
        const batteryService = {
          uuid: 'battery_service',
          getCharacteristic: (name) => {
            if (name === 'battery_level') return Promise.resolve(batteryCharacteristic)
            return Promise.reject(new Error('characteristic unavailable'))
          }
        }

        const txListeners = new Set()
        const txCharacteristic = {
          properties: { notify: true, indicate: true, read: true },
          startNotifications: () => Promise.resolve(txCharacteristic),
          stopNotifications: () => Promise.resolve(),
          addEventListener: (_name, listener) => txListeners.add(listener),
          removeEventListener: (_name, listener) => txListeners.delete(listener)
        }
        const emitTx = (bytes) => {
          const event = { target: { value: asDataView(bytes) } }
          txListeners.forEach((listener) => listener(event))
        }
        const rxCharacteristic = {
          properties: { write: true, writeWithoutResponse: true, read: true },
          writeValueWithoutResponse: () => {
            setTimeout(() => emitTx(Uint8Array.from([0xD2, 0x03, 0x01, 0x3D])), 0)
            return Promise.resolve()
          }
        }
        const dalyService = {
          uuid: DALY_NUS_SERVICE_UUID,
          getCharacteristic: (uuid) => {
            const normalized = String(uuid || '').toLowerCase()
            if (normalized === DALY_NUS_TX_UUID) return Promise.resolve(txCharacteristic)
            if (normalized === DALY_NUS_RX_UUID) return Promise.resolve(rxCharacteristic)
            return Promise.reject(new Error('characteristic unavailable'))
          }
        }

        const gattServer = {
          getPrimaryService: (name) => {
            const normalized = String(name || '').toLowerCase()
            if (normalized === DALY_NUS_SERVICE_UUID) return Promise.resolve(dalyService)
            if (normalized === 'battery_service') return Promise.resolve(batteryService)
            return Promise.reject(new Error('service unavailable'))
          },
          getPrimaryServices: () => Promise.resolve([dalyService, batteryService])
        }

        Object.defineProperty(win.navigator, 'bluetooth', {
          configurable: true,
          value: {
            requestDevice: () => Promise.resolve({
              id: 'daly-test-01',
              name: 'DL-MyPack',
              gatt: {
                connected: false,
                connect: () => Promise.resolve(gattServer)
              }
            }),
            getDevices: () => Promise.resolve([])
          }
        })

        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-open-daly_bms"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-daly_bms"]').should('be.visible')

    cy.get('[data-testid="daly-select-device"]').click({ force: true })
    cy.contains('Selected DL-MyPack via Web Bluetooth.').should('exist')

    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-verify-daly_bms"]').click({ force: true })
    cy.get('[data-testid="integration-stepper-success"]').should('be.visible')

    cy.get('[data-testid="daly-run-probe"]').click({ force: true })
    cy.get('[data-testid="daly-probe-summary"]', { timeout: 12000 }).should('contain.text', 'Probe ok')
  })
})
