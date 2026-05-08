# ESEG v1

ESEG is the SDK distributed segment manifest record.

On disk, `segment.eseg` is a rkyv archive of:

```text
SdkWireRecord::Segment(SegmentRecord)
```

The old compact byte format with an `ESEG` magic header has been removed. Do not
add a compatibility parser for it.

A segment binds a node role, capability label, composition hash, input/output
arity, and component/step range. It does not define a transport. The editable
source is `segment.edsl`.
