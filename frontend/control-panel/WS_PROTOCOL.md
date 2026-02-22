# Control Panel WebSocket Protocol (Pinned)

Path: `/api/ws`  
Version identifier: `edgerun.control_panel.ws.v1`

This file is the authoritative contract for `frontend/control-panel/src/services/api.js`.

## 1) Client -> Server request envelope

```json
{
  "request_id": "<non-empty string>",
  "op": "status" | "run",
  "protocol": "edgerun.control_panel.ws.v1",
  "token": "<optional string>",
  "payload": { ... }
}
```

Rules:
- `request_id` is mandatory and unique per in-flight request.
- `op=status` requires `payload={}`.
- `op=run` requires `payload.task` as non-empty string.

## 2) Server -> Client response envelope

```json
{
  "request_id": "<same id>",
  "ok": true | false,
  "data": { ... },
  "error": "<message>",
  "status": 400
}
```

Rules:
- `request_id` is mandatory and must match an in-flight request.
- `ok=true` returns `data`.
- `ok=false` returns `error` (and optional numeric `status`).

## 3) Server -> Client push event (canonical)

```json
{
  "event": "status",
  "data": {
    "tasks": [
      {
        "task": "doctor",
        "state": "idle",
        "runs": 0,
        "last_exit": null,
        "last_output": ""
      }
    ]
  }
}
```

Rules:
- Canonical push shape is `event=status` with `data.tasks[]`.
- `tasks[].task` is required. Other task fields are optional and normalized client-side.

## 4) Status body

```json
{
  "tasks": [
    {
      "task": "<non-empty string>",
      "state": "<string>",
      "runs": 0,
      "last_exit": 0,
      "last_output": "<string>"
    }
  ]
}
```

## Compatibility note

For backward compatibility, the client still accepts legacy push status variants:
- direct status body `{ "tasks": [...] }`
- status-tagged envelopes using `type="status"` or `op="status"` with `data.tasks`

These legacy forms are tolerated but not canonical.

## JSON Schema

Machine-readable schema is pinned at:
- `frontend/control-panel/schema/control-panel-ws-v1.schema.json`
