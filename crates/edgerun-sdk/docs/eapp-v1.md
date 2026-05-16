# EAPP v1

EAPP is the SDK app artifact graph record.

On disk, `app.eapp` is a rkyv archive of:

```text
SdkWireRecord::AppGraph(AppGraphRecord)
```

The old compact byte format with an `EAPP` magic header has been removed. Do not
add a compatibility parser for it.

The graph binds packaged app files, artifact kinds, sizes, and SHA-256 hashes.
Signatures over app packages must bind these exact rkyv bytes.
