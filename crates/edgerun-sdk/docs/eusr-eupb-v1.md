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

The profile file encrypts the signed body. The body binds profile id, owner,
epoch, capability grants, and owner signature material.
