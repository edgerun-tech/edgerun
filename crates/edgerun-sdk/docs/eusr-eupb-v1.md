# EUSR/EUPB v1

EUSR is the SDK encrypted user profile record. EUPB is the owner-signed profile
body record.

On disk, profile artifacts are rkyv archives of:

```text
SdkWireRecord::UserProfile(UserProfile)
SdkWireRecord::UserProfileBody(UserProfileBody)
```

The old compact byte formats with `EUSR` and `EUPB` magic headers have been
removed. Do not add compatibility parsers for them.

The profile file encrypts the signed body. The outer profile header carries the
password KDF metadata needed to derive the seal key. The body binds profile id,
owner, epoch, capability grants, the Ed25519 owner seed, and owner signature
material.

The sealed owner seed is the user's identity root after unlock. Browser and
native storage targets should store the `.eusr` bytes directly, or a lossless
encoding of those bytes such as base64 for `localStorage`. Do not translate the
profile into JSON or another canonical wire format.

Low-level tooling may still seal with a raw 32-byte seal key. User onboarding
should use the password path:

```text
edgerun-sdk create-user-profile-password profile.eusr <owner-seed-hex> <password> <epoch> [monotonic-version]
edgerun-sdk open-user-profile-password profile.eusr <password>
```
