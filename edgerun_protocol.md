# Edgerun Protocol

The internal protocol wire is rkyv only.

Legacy schema text and generated-schema canonicalization rules were removed to
avoid ambiguity. Protocol records must be archived as concrete rkyv types for
hashing, signing, storage, caches, transports, and local bridge payloads.
