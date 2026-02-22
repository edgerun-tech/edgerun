import fs from 'node:fs'
import path from 'node:path'

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..')
const schemaPath = path.join(root, 'control-panel', 'schema', 'control-panel-ws-v1.schema.json')
const protocolDocPath = path.join(root, 'control-panel', 'WS_PROTOCOL.md')
const apiPath = path.join(root, 'control-panel', 'src', 'services', 'api.js')

function fail(message) {
  console.error(`control-panel ws schema check failed: ${message}`)
  process.exit(1)
}

function assert(condition, message) {
  if (!condition) fail(message)
}

function readJson(filePath) {
  try {
    const raw = fs.readFileSync(filePath, 'utf8')
    return JSON.parse(raw)
  } catch (err) {
    fail(`cannot parse JSON at ${filePath}: ${String(err.message || err)}`)
  }
}

function isRecord(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function isNonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0
}

function isTask(value) {
  return isRecord(value) && isNonEmptyString(value.task)
}

function isStatusBody(value) {
  return isRecord(value) && Array.isArray(value.tasks) && value.tasks.every(isTask)
}

function isClientRequest(value) {
  if (!isRecord(value)) return false
  if (!isNonEmptyString(value.request_id)) return false
  if (!['status', 'run'].includes(value.op)) return false
  if (value.protocol !== 'edgerun.control_panel.ws.v1') return false
  if (!isRecord(value.payload)) return false

  if (value.op === 'status') return Object.keys(value.payload).length === 0
  if (value.op === 'run') return isNonEmptyString(value.payload.task)
  return false
}

function isServerResponse(value) {
  if (!isRecord(value)) return false
  if (!isNonEmptyString(value.request_id)) return false
  if (typeof value.ok !== 'boolean') return false
  if (value.ok) return true
  return isNonEmptyString(value.error)
}

function isServerStatusPush(value) {
  return isRecord(value) && value.event === 'status' && isStatusBody(value.data)
}

const schema = readJson(schemaPath)
const protocolDoc = fs.readFileSync(protocolDocPath, 'utf8')
const apiJs = fs.readFileSync(apiPath, 'utf8')

assert(schema.$id === 'https://edgerun.tech/schemas/control-panel-ws-v1.schema.json', 'unexpected schema $id')
assert(schema.$schema === 'https://json-schema.org/draft/2020-12/schema', 'unexpected JSON schema draft')
assert(schema.$defs && schema.$defs.clientRequest && schema.$defs.serverResponse && schema.$defs.serverStatusPush, 'missing required schema defs')

const requestOpEnum = schema.$defs.clientRequest?.properties?.op?.enum
assert(Array.isArray(requestOpEnum), 'client request op enum must exist')
assert(requestOpEnum.includes('status') && requestOpEnum.includes('run'), 'client request op enum must include status and run')

const protocolConst = schema.$defs.clientRequest?.properties?.protocol?.const
assert(protocolConst === 'edgerun.control_panel.ws.v1', 'client request protocol const mismatch')
assert(apiJs.includes(`CONTROL_PANEL_WS_PROTOCOL_VERSION = '${protocolConst}'`), 'api.js protocol constant must match schema')
assert(protocolDoc.includes(protocolConst), 'WS_PROTOCOL.md must mention protocol version')
assert(protocolDoc.includes('/api/ws'), 'WS_PROTOCOL.md must mention /api/ws endpoint')

const validClientMessages = [
  {
    request_id: 'r-1',
    op: 'status',
    protocol: 'edgerun.control_panel.ws.v1',
    token: '',
    payload: {}
  },
  {
    request_id: 'r-2',
    op: 'run',
    protocol: 'edgerun.control_panel.ws.v1',
    token: 'abc',
    payload: { task: 'doctor' }
  }
]
for (const message of validClientMessages) {
  assert(isClientRequest(message), `invalid canonical client message: ${JSON.stringify(message)}`)
}

const validServerMessages = [
  { request_id: 'r-1', ok: true, data: { tasks: [{ task: 'doctor' }] } },
  { request_id: 'r-2', ok: false, error: 'request failed', status: 400 }
]
for (const message of validServerMessages) {
  assert(isServerResponse(message), `invalid canonical server response: ${JSON.stringify(message)}`)
}

const validPush = {
  event: 'status',
  data: {
    tasks: [
      {
        task: 'doctor',
        state: 'idle',
        runs: 0,
        last_exit: null,
        last_output: ''
      }
    ]
  }
}
assert(isServerStatusPush(validPush), 'invalid canonical server status push message')

const invalidMessages = [
  { request_id: '', op: 'status', protocol: 'edgerun.control_panel.ws.v1', payload: {} },
  { request_id: 'x', op: 'status', protocol: 'v0', payload: {} },
  { request_id: 'x', op: 'run', protocol: 'edgerun.control_panel.ws.v1', payload: {} },
  { request_id: 'x', ok: false },
  { event: 'status', data: { tasks: [{ task: '' }] } }
]
assert(!isClientRequest(invalidMessages[0]), 'invalid client request unexpectedly accepted')
assert(!isClientRequest(invalidMessages[1]), 'invalid protocol unexpectedly accepted')
assert(!isClientRequest(invalidMessages[2]), 'invalid run payload unexpectedly accepted')
assert(!isServerResponse(invalidMessages[3]), 'invalid response unexpectedly accepted')
assert(!isServerStatusPush(invalidMessages[4]), 'invalid push unexpectedly accepted')

console.log('control-panel ws schema check passed')
