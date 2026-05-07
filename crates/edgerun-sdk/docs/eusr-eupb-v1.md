# EUSR/EUPB v1

EUSR is the encrypted user trust profile artifact. EUPB is the decrypted,
owner-signed profile body. These are SDK artifact formats, not Edgerun internal
wire, storage, cache, or local bridge protocols.

The storage location is not authority. Google Drive, browser local storage,
filesystem, remote storage, and email attachments only carry the encrypted EUSR
bytes. Authority comes from decrypting the body locally and verifying the owner
signature.

## EUSR

```text
offset  size  field
0       4     magic = "EUSR"
4       2     profile_version = 1
6       2     sdk_abi_version = 2
8       4     flags
12      4     sealed_body_len
16      8     monotonic_version
24      32    profile_id
56      32    owner_id
88      32    body_sha256
120     N     sealed EUPB bytes
```

`sealed_body` is produced by the runtime sealing capability. The SDK CLI uses
`edgerun-seal` with AES-GCM under a 256-bit local runtime key.

## EUPB

```text
offset  size  field
0       4     magic = "EUPB"
4       2     body_version = 1
6       2     sdk_abi_version = 2
8       4     flags
12      2     grant_count
14      2     reserved = 0
16      8     epoch
24      8     monotonic_version
32      32    profile_id
64      32    owner_id
96      ...   grants
...     2     signature_len
...     S     owner signature
```

Owner signature input:

```text
"edgerun-sdk.eupb.v1.user-profile-body" || sha256(unsigned EUPB body)
```

## Grant

```text
offset  size  field
0       2     capability_kind
2       2     operation
4       2     min_assurance
6       2     flags
8       8     valid_from
16      8     valid_until
24      32    app_id
56      32    release_id, or zero for any release
88      32    scope_sha256, or zero for any scope
```

For rkyv request access checks:

- request app id must match grant app id
- request release id must match, unless grant release id is zero
- request capability kind and operation must match
- request assurance must be at least `min_assurance`
- check time must be inside the validity window
- scope matches when `scope_sha256 == sha256(request.context)`, unless zero

## CLI

```text
create-user-profile <out.eusr> <owner-seed-hex> <seal-key-hex> <epoch> <version>
grant-profile-capability <in.eusr> <out.eusr> <seal-key-hex> <owner-seed-hex> \
  <app-id> <release-id|any> <kind> <operation> <scope-hash|any|context:hex> \
  <min-assurance> <valid-from> <valid-until> <new-version>
open-user-profile <profile.eusr> <seal-key-hex>
verify-profile-access <profile.eusr> <seal-key-hex> <request.rkyv> <at>
sign-request-authorized <profile.eusr> <profile-key-hex> <request.rkyv> \
  <out.rkyv> <provider> <signer-seed-hex> <at> [assurance]
seal-request-authorized <profile.eusr> <profile-key-hex> <request.rkyv> \
  <out.rkyv> <provider> <seal-key-hex> <at> [assurance]
unseal-request-authorized <profile.eusr> <profile-key-hex> <request.rkyv> \
  <out.rkyv> <provider> <seal-key-hex> <at> [assurance]
storage-read-request-authorized <profile.eusr> <profile-key-hex> <request.rkyv> \
  <out.rkyv> <provider> <root-dir> <at> [assurance]
storage-write-request-authorized <profile.eusr> <profile-key-hex> <request.rkyv> \
  <out.rkyv> <provider> <root-dir> <at> [assurance]
```

The authorized execution commands run the same profile check before invoking the
runtime capability. If no grant matches, no successful capability response is
written.
