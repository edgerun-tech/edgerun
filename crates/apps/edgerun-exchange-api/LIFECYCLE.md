# Exchange API App Lifecycle

This is the current concrete path for one app-shaped crate.

## 1. App Crate Declares Runtime Intent

`edgerun-exchange-api` exposes runtime intent through SDK records:

- `runtime_app_projection(developer_id, host)` builds the current
  `RuntimeAppInstall` wire record for a verified runtime projection.
- `declared_routes(developer_id, host)` builds `RuntimeHttpRoute` records.
- `STORAGE_NAMESPACE` declares the app-owned storage namespace.
- `build_handler()` returns an HTTP handler but does not bind a listener.

The app crate should stay on this side of the boundary: deterministic app
identity, declared routes, declared storage/capability use, and request handler
logic.

## 2. Node Records A Verified Runtime Projection

`edgerun-node::runtime::RuntimeKernel::record_app_runtime_projection` records a
verified runnable app projection. It is a local runtime state transition, not
network authority.

The node records the legacy ABI event code `RUNTIME_EVENT_APP_INSTALLED` with
the rkyv archived `RuntimeAppInstall` payload. In the current model that event
means "verified runtime projection recorded", not app installation authority.
After this event, the append-only event log is local audit/replay evidence for
the app identity, release, manifest hash, declared routes, storage namespaces,
and capability declarations. Cross-node authority should come from admitted work
and its proofs.

## 3. Node Grants Declared Routes

`RuntimeKernel::grant_http_route` only accepts routes already present in the
verified app runtime declaration. A route not declared by the app is rejected
and logged as a denied route grant.

For exchange API, the initial declared HTTPS prefixes are:

- `/v1/quote`
- `/v1/payment-request`
- `/v1/order`
- `/v1/assets`
- `/health`

## 4. Node Owns Listener And Dispatch

`edgerun-node::services::http_runtime::HttpNodeBinding` owns the socket bind.
It currently accepts a connection and logs the target app id, but returns:

```text
app ipc http dispatch is not wired yet
```

The intended next step is to replace that placeholder with:

1. parse the HTTP request into `RuntimeHttpRequest`,
2. call `RuntimeKernel::dispatch_http`,
3. route the accepted dispatch to the verified app handler/process,
4. append the dispatch event before app execution,
5. append app result/capability events through node-owned APIs.

## 5. Node Brokers Storage And Capabilities

Exchange API currently has an in-crate `ExchangeStore` for local route logic.
The runtime boundary should move durable state through node-mediated storage:

- app sends a rkyv `CapabilityRequest`,
- node validates app id, release id, namespace, operation, and payload hash,
- node appends `RUNTIME_EVENT_CAPABILITY_EXECUTED` or denied event,
- node storage/event log remains local audit evidence and projection state.

## Current Gap

The run/route/storage projection model exists in `edgerun-node::runtime`, and
the app can declare runtime records through `edgerun-sdk::runtime_api`. The
missing bridge is the live app runner: `HttpNodeBinding` does not yet convert
accepted HTTP frames into `RuntimeHttpRequest` plus app IPC/handler invocation.
