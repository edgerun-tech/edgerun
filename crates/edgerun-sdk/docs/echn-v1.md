# ECHN v1

ECHN is the SDK distributed chain manifest record.

On disk, `chain.echn` is a rkyv archive of:

```text
SdkWireRecord::Chain(ChainRecord)
```

The old compact byte format with an `ECHN` magic header has been removed. Do not
add a compatibility parser for it.

A chain declares the ordered segment set for a distributed app and hash links
between segment outputs and later segment inputs. It does not define transport.
The editable source is `chain.edsl`.
