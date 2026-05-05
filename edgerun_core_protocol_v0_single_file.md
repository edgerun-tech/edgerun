# Edgerun Core Protocol v0

The internal protocol wire is rkyv only.

This file intentionally replaces the previous schema-canonical specification so
there is no second internal protocol definition. Hashing, signing, storage,
transport payloads, caches, and local bridge payloads must archive concrete
protocol records through the rkyv boundary.
