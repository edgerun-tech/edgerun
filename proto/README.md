# Proto layout

The protocol spec embeds authoritative `proto3` snippets for these schema files:

- `edgerun/v0/common.proto`
- `edgerun/v0/identity.proto`
- `edgerun/v0/trust.proto`
- `edgerun/v0/stream.proto`
- `edgerun/v0/object.proto`
- `edgerun/v0/access.proto`
- `edgerun/v0/network.proto`

To check that the spec-defined proto files remain complete with respect to the embedded spec snippets, run:

```bash
./scripts/audit_proto_against_spec.py
```

Current expectation:

- the seven spec-defined proto files must include every enum value and message field declared in the spec
