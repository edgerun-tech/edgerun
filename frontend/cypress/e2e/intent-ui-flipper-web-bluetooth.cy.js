// SPDX-License-Identifier: Apache-2.0
import { installLocalBridgeSimulator } from '../helpers/local-bridge-simulator'

describe('intent ui flipper web bluetooth integration', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_flipper_test')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify(['openid', 'profile', 'edgerun:profile.read', 'edgerun:profile.write'])
    )
  }

  it('selects flipper via web bluetooth and opens workflow bootstrap', () => {
    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        installLocalBridgeSimulator(win)
        win.localStorage.removeItem('intent-ui-integrations-v1')
        win.localStorage.removeItem('flipper_device_id')
        win.localStorage.removeItem('flipper_device_name')

        const SERIAL_SERVICE_UUID = '8fe5b3d5-2e7f-4a98-2a48-7acc60fe0000'
        const SERIAL_TX_UUID = '19ed82ae-ed21-4c9d-4145-228e61fe0000'
        const SERIAL_RX_UUID = '19ed82ae-ed21-4c9d-4145-228e62fe0000'
        const SERIAL_FLOW_UUID = '19ed82ae-ed21-4c9d-4145-228e63fe0000'
        const SERIAL_RPC_STATUS_UUID = '19ed82ae-ed21-4c9d-4145-228e64fe0000'

        const toBytes = (input) => {
          if (input instanceof Uint8Array) return input
          if (Array.isArray(input)) return Uint8Array.from(input)
          if (typeof input === 'string') return new win.TextEncoder().encode(input)
          if (input instanceof ArrayBuffer) return new Uint8Array(input)
          return new Uint8Array()
        }
        const asDataView = (input) => {
          const bytes = toBytes(input)
          return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
        }
        const concat = (chunks) => {
          const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0)
          const out = new Uint8Array(total)
          let offset = 0
          for (const chunk of chunks) {
            out.set(chunk, offset)
            offset += chunk.length
          }
          return out
        }
        const encodeVarint = (value) => {
          let n = Number(value || 0)
          const out = []
          while (n >= 0x80) {
            out.push((n & 0x7f) | 0x80)
            n = Math.floor(n / 128)
          }
          out.push(n & 0x7f)
          return Uint8Array.from(out)
        }
        const decodeVarint = (bytes, offset = 0) => {
          let value = 0
          let shift = 0
          let cursor = offset
          while (cursor < bytes.length) {
            const byte = bytes[cursor]
            value += (byte & 0x7f) * (2 ** shift)
            cursor += 1
            if ((byte & 0x80) === 0) return { value, offset: cursor }
            shift += 7
          }
          throw new Error('invalid varint')
        }
        const tryDecodeDelimitedFrames = (buffer) => {
          const frames = []
          let cursor = 0
          try {
            while (cursor < buffer.length) {
              const len = decodeVarint(buffer, cursor)
              const start = len.offset
              const end = start + len.value
              if (end > buffer.length) break
              frames.push(buffer.slice(start, end))
              cursor = end
            }
          } catch {
            return { frames: [], remainder: buffer }
          }
          return { frames, remainder: buffer.slice(cursor) }
        }
        const decodeMain = (frameBytes) => {
          let commandId = 0
          let contentTag = 0
          let contentBytes = new Uint8Array()
          let cursor = 0
          while (cursor < frameBytes.length) {
            const key = decodeVarint(frameBytes, cursor)
            cursor = key.offset
            const tag = key.value >> 3
            const wireType = key.value & 0x07
            if (wireType === 0) {
              const field = decodeVarint(frameBytes, cursor)
              cursor = field.offset
              if (tag === 1) commandId = field.value
              continue
            }
            if (wireType === 2) {
              const length = decodeVarint(frameBytes, cursor)
              const start = length.offset
              const end = start + length.value
              contentTag = tag
              contentBytes = frameBytes.slice(start, end)
              cursor = end
              continue
            }
            break
          }
          return { commandId, contentTag, contentBytes }
        }
        const decodePingRequestData = (contentBytes) => {
          let cursor = 0
          while (cursor < contentBytes.length) {
            const key = decodeVarint(contentBytes, cursor)
            cursor = key.offset
            const tag = key.value >> 3
            const wireType = key.value & 0x07
            if (wireType === 2) {
              const length = decodeVarint(contentBytes, cursor)
              const start = length.offset
              const end = start + length.value
              const bytes = contentBytes.slice(start, end)
              cursor = end
              if (tag === 1) return bytes
              continue
            }
            if (wireType === 0) {
              const field = decodeVarint(contentBytes, cursor)
              cursor = field.offset
              continue
            }
            break
          }
          return new Uint8Array()
        }
        const encodeField = (tag, wireType, payload) => concat([encodeVarint((tag << 3) | wireType), payload])
        const encodeBytesField = (tag, payload) => {
          const bytes = toBytes(payload)
          return encodeField(tag, 2, concat([encodeVarint(bytes.length), bytes]))
        }
        const encodeBoolField = (tag, value) => encodeField(tag, 0, Uint8Array.from([value ? 1 : 0]))
        const encodeUintField = (tag, value) => encodeField(tag, 0, encodeVarint(value))
        const encodeMain = ({ commandId, contentTag, contentBytes }) =>
          concat([
            encodeUintField(1, commandId),
            encodeBoolField(3, false),
            encodeField(contentTag, 2, concat([encodeVarint(contentBytes.length), contentBytes]))
          ])
        const encodeDelimited = (payload) => concat([encodeVarint(payload.length), payload])

        const batteryCharacteristic = {
          readValue: () => Promise.resolve(asDataView([87]))
        }

        const txListeners = new Set()
        const emitTx = (bytes) => {
          const event = { target: { value: asDataView(bytes) } }
          txListeners.forEach((listener) => listener(event))
        }
        const txCharacteristic = {
          properties: { read: true, indicate: true },
          startNotifications: () => Promise.resolve(txCharacteristic),
          stopNotifications: () => Promise.resolve(),
          addEventListener: (_name, listener) => txListeners.add(listener),
          removeEventListener: (_name, listener) => txListeners.delete(listener)
        }
        const flowListeners = new Set()
        const flowCharacteristic = {
          properties: { read: true, notify: true },
          readValue: () => Promise.resolve(asDataView([0, 0, 1, 0])),
          startNotifications: () => Promise.resolve(flowCharacteristic),
          stopNotifications: () => Promise.resolve(),
          addEventListener: (_name, listener) => flowListeners.add(listener),
          removeEventListener: (_name, listener) => flowListeners.delete(listener)
        }
        const rpcStatusCharacteristic = {
          properties: { read: true, write: true, notify: true }
        }
        const rxCharacteristic = {
          properties: { read: true, write: true, writeWithoutResponse: true },
          _inboundBuffer: new Uint8Array(),
          writeValueWithoutResponse: (payload) => {
            rxCharacteristic._inboundBuffer = concat([rxCharacteristic._inboundBuffer, toBytes(payload)])
            const decoded = tryDecodeDelimitedFrames(rxCharacteristic._inboundBuffer)
            rxCharacteristic._inboundBuffer = decoded.remainder
            decoded.frames.forEach((frame) => {
              const request = decodeMain(frame)
              if (request.contentTag === 5) {
                const pingRequestData = decodePingRequestData(request.contentBytes)
                const pingResponseBody = encodeBytesField(1, pingRequestData)
                const responseMain = encodeMain({
                  commandId: request.commandId,
                  contentTag: 6,
                  contentBytes: pingResponseBody
                })
                setTimeout(() => emitTx(encodeDelimited(responseMain)), 0)
              } else if (request.contentTag === 32) {
                const entries = [
                  ['model_name', 'Flipper Zero'],
                  ['firmware_version', 'dev']
                ]
                entries.forEach(([key, value], index) => {
                  const infoBody = concat([encodeBytesField(1, key), encodeBytesField(2, value)])
                  const frameMain = concat([
                    encodeUintField(1, request.commandId),
                    encodeUintField(2, 0),
                    encodeBoolField(3, index < entries.length - 1),
                    encodeField(33, 2, concat([encodeVarint(infoBody.length), infoBody]))
                  ])
                  setTimeout(() => emitTx(encodeDelimited(frameMain)), 0)
                })
              }
            })
            return Promise.resolve()
          }
        }
        const serialService = {
          uuid: SERIAL_SERVICE_UUID,
          getCharacteristic: (uuid) => {
            const key = String(uuid || '').toLowerCase()
            if (key === SERIAL_TX_UUID) return Promise.resolve(txCharacteristic)
            if (key === SERIAL_RX_UUID) return Promise.resolve(rxCharacteristic)
            if (key === SERIAL_FLOW_UUID) return Promise.resolve(flowCharacteristic)
            if (key === SERIAL_RPC_STATUS_UUID) return Promise.resolve(rpcStatusCharacteristic)
            return Promise.reject(new Error('characteristic unavailable'))
          }
        }
        const batteryService = {
          uuid: 'battery_service',
          getCharacteristic: (name) => {
            if (name === 'battery_level') return Promise.resolve(batteryCharacteristic)
            return Promise.reject(new Error('characteristic unavailable'))
          }
        }
        const gattServer = {
          getPrimaryService: (name) => {
            const key = String(name || '').toLowerCase()
            if (key === SERIAL_SERVICE_UUID) return Promise.resolve(serialService)
            if (name === 'battery_service') return Promise.resolve(batteryService)
            return Promise.reject(new Error('service unavailable'))
          },
          getPrimaryServices: () => Promise.resolve([serialService, batteryService])
        }
        const bluetooth = {
          requestDevice: () => Promise.resolve({
            id: 'flipper-test-01',
            name: 'Flipper Zero',
            gatt: {
              connected: false,
              connect: () => Promise.resolve(gattServer)
            }
          }),
          getDevices: () => Promise.resolve([])
        }
        Object.defineProperty(win.navigator, 'bluetooth', {
          configurable: true,
          value: bluetooth
        })

        seedProfileSession(win)
      }
    })

    cy.get('button[title="Integrations panel"]').first().click({ force: true })
    cy.get('[data-testid="provider-open-flipper"]').click({ force: true })
    cy.get('[data-testid="provider-dialog-flipper"]').should('be.visible')

    cy.get('[data-testid="integration-step-1"]').click({ force: true })
    cy.get('[data-testid="flipper-select-device"]').click({ force: true })
    cy.contains('Selected Flipper Zero via Web Bluetooth.').should('exist')

    cy.get('[data-testid="integration-step-2"]').click({ force: true })
    cy.get('[data-testid="provider-verify-flipper"]').click({ force: true })
    cy.get('[data-testid="integration-stepper-success"]').should('be.visible')
    cy.get('[data-testid="flipper-run-probe"]').click({ force: true })
    cy.get('[data-testid="flipper-probe-summary"]', { timeout: 12000 }).should('contain.text', 'Probe ok')
    cy.get('[data-testid="flipper-probe-details"]').should('contain.text', 'Model: Flipper Zero')
    cy.get('[data-testid="flipper-create-workflow"]').click({ force: true })

    cy.window().then((win) => {
      const state = win.__intentDebug.getWorkflowUi()
      expect(state.isOpen).to.eq(true)
      expect(state.selectedIntegrationId).to.eq('flipper')
    })
  })
})
