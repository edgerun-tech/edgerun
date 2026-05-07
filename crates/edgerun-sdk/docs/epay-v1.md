# EPAY v1

EPAY is the SDK payment record.

On disk, payment artifacts are rkyv archives of:

```text
SdkWireRecord::Payment(PaymentRecord)
```

The old compact byte format with an `EPAY` magic header has been removed. Do not
add a compatibility parser for it.

A payment record binds payer intent, app/product context, amount, currency,
purpose, and settlement signature material.
