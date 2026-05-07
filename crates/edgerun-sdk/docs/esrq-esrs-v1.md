# ESRQ/ESRS v1

ESRQ/ESRS has been folded into the generic rkyv capability invocation records.
See `ecap-ecrp-v1.md`.

The signing CLI commands still exist, but they now write:

- `write-sign-request`: rkyv capability signing request
- `sign-request`: rkyv capability signing response
- `verify-sign-response`: signing proof verification

Old ESRQ/ESRS and ECAP/ECRP byte artifacts are intentionally not compatibility
paths.
