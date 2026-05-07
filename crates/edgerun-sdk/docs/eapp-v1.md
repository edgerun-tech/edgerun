# EAPP v1

EAPP is the compact binary artifact graph for an Edgerun app package. It is an
SDK artifact format, not an Edgerun internal wire, storage, cache, or local
bridge protocol.

`app.edapp` remains the browser-readable launch manifest. `app.eapp` is the
deterministic graph that gets signed by the developer and optionally by a store.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EAPP"
4       2     app_graph_version = 1
6       2     abi_version = 2
8       4     flags
12      2     artifact_count
14      2     app_slug_len
16      32    app_id
48      32    developer_public_key
80      32    app_manifest_sha256
112     N     app_slug
112+N   ...   artifact records
```

Flags:

```text
bit 0: deterministic artifact graph
```

App identity:

```text
app_id = sha256("edgerun-sdk.eapp.v1.app-id" || developer_public_key || app_slug)
```

Each artifact record:

```text
offset  size  field
0       2     artifact_kind
2       2     path_len
4       32    sha256
36      N     package-relative path bytes
```

Artifact kinds:

```text
0 unknown/supporting artifact
1 app.edapp
2 Edgerun binary metadata or composition artifact
3 wasm implementation
4 native implementation blob
5 browser asset
```

Developer signature:

```text
developer.esig = Ed25519 ESIG over
"edgerun-sdk.esig.v1.app.developer" || sha256(app.eapp)
```

Store signature:

```text
store.esig = Ed25519 ESIG over
"edgerun-sdk.esig.v1.app.store" || sha256(app.eapp)
```

The store signature is an approval/distribution receipt for the exact same app
graph. It does not replace the developer signature and does not become the
source of truth for app contents.
