# Protocol Type Audit - Dashboard Shadow Protocol

**Status**: RESOLVED - Shadow protocol interfaces removed.

**Date**: 2026-05-02

## Generation Workflow

### Prerequisites
- `protoc` installed (v3.21+)
- `protoc-gen-ts` npm package installed globally: `npm install -g protoc-gen-ts`

### Generate TypeScript Proto Types
```bash
# From edgerun_core root:
protoc --ts_out=crates/edgerun-dash-webapp/gen \
  --proto_path=proto \
  proto/edgerun/v0/common.proto \
  proto/edgerun/v0/stream.proto \
  proto/edgerun/v0/capability.proto \
  proto/edgerun/v0/trust.proto \
  proto/edgerun/v0/object.proto
```

Generated files:
- `crates/edgerun-dash-webapp/gen/edgerun/v0/common.ts`
- `crates/edgerun-dash-webapp/gen/edgerun/v0/stream.ts`
- `crates/edgerun-dash-webapp/gen/edgerun/v0/capability.ts`
- `crates/edgerun-dash-webapp/gen/edgerun/v0/trust.ts`
- `crates/edgerun-dash-webapp/gen/edgerun/v0/object.ts`

### Guardrail Check
Run this to verify no shadow protocol interfaces exist:
```bash
rg "interface (ObjectRef|EventRef|CommandRef|NodeRef|IdentityRef|AppPackage|AppPrincipal|CapabilityGrant|CapabilityDescriptor|DelegationRecord|CommandEnvelope|StreamEvent)" \
  crates/edgerun-dash-webapp/platform/ \
  crates/edgerun-dash-webapp/stores/ \
  crates/edgerun-dash-webapp/components/
```
Should return nothing.

## Changes Made

### Protocol Files Refactored
| File | Before | After |
|------|--------|-------|
| refs.ts | Local ObjectRef, EventRef, etc. | Imports from gen/edgerun/v0/common |
| commands.ts | Manual CommandEnvelope, encode/sign | Uses generated CommandEnvelope, serialize/deserialize |
| apps.ts | Local AppPackage, AppRoute, etc. | Uses generated AppPackage |
| capabilities.ts | Local CapabilityDescriptor, etc. | Uses generated types |
| streams.ts | Local StreamEvent, manual hashing | Uses generated EventEnvelope |

### Stores Updated
| File | Before | After |
|------|--------|-------|
| app-store.ts | Map<string, AppPackage> (local) | Map<string, stream.AppPackage> |
| capability-store.ts | Map<string, CapabilityDescriptor> (local) | Map<string, cap.CapabilityDescriptor> |
| command-store.ts | Local CommandEnvelope | stream.CommandEnvelope |

### Naming Convention
- Protocol types: imported from gen/edgerun/v0/*
- UI view models: end in View
- Form/editor drafts: end in Draft
- JSON transport: end in Json

## Remaining Work
- Wire protobuf serialization for command sending/receiving
- Regenerate types when proto files change
- Add CI check to prevent shadow interfaces
