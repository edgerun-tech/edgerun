# Capability Units v1

The SDK capability units mirror the existing Edgerun capability crates:

- `edgerun-capabilities` defines descriptors, selectors, requests, grants,
  invocations, results, revocations, and policy authorization.
- `edgerun-remote-capability` defines session open, invocation, result frames,
  events, close, and request/grant/revocation transport envelopes.

The internal payloads remain Edgerun rkyv values. These wasm units define the
deterministic invocation boundaries and scalar checks that apps can compose
before calling a real provider capability.

## Units

`capability-policy-v1` exports:

```text
capability_operation_valid(operation) -> bool
capability_access_class_valid(access_class) -> bool
capability_authorize_invocation(granted_ops_ptr, granted_ops_count, operation,
  requested_access_class, is_local, grant_revoked) -> status
```

`capability-session-v1` exports:

```text
capability_session_mode_valid(mode) -> bool
capability_session_open_validate(version, session_id_len, mode,
  requested_ops_count, requested_access_class) -> status
capability_session_accept_unchecked_status(open_status) -> status
capability_session_reject_status() -> status
```

`capability-provider-v1` exports:

```text
capability_invocation_validate(version, invocation_id_len, grant_id_len,
  operation, requested_access_class) -> status
capability_result_validate(version, invocation_id_len, grant_id_len,
  result_access_class) -> status
capability_result_frame_validate(result_present, inline_payload_len) -> status
```

Status `0` is success. Non-zero statuses are deterministic rejects.
