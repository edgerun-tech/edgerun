use super::helpers::*;
pub fn validate_object_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
) -> ValidationResult {
    if let Some(descriptor) = get_map(semantic_input, "descriptor") {
        let header = get_map(semantic_input, "header");
        let manifest = get_map(semantic_input, "chunk_manifest");
        let descriptor_object_id = string_value(descriptor, "object_id", "");
        if let Some(header) = header {
            let header_object = get_map(header, "object")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
            if !descriptor_object_id.is_empty()
                && !header_object.is_empty()
                && descriptor_object_id != header_object
            {
                return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
            }
            let chunking_mode = string_value(header, "chunking_mode", "");
            if chunking_mode == "CHUNKING_MODE_MANIFEST" && manifest.is_none() {
                return defer(
                    ReasonCode::MissingDependency,
                    mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                );
            }
        }
        if let Some(manifest) = manifest {
            let manifest_object = get_map(manifest, "object")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
            if !descriptor_object_id.is_empty()
                && !manifest_object.is_empty()
                && descriptor_object_id != manifest_object
            {
                return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
            }
            let entries = get_seq(manifest, "entries")
                .or_else(|| get_seq(manifest, "chunk_entries"))
                .unwrap_or(&[]);
            let claimed_count = manifest
                .get("chunk_count")
                .and_then(Value::as_i64)
                .unwrap_or(entries.len() as i64);
            if claimed_count != entries.len() as i64 {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            let total_len: i64 = entries
                .iter()
                .filter_map(Value::as_map)
                .map(|m| number_value(m, "length", 0))
                .sum();
            let claimed_total = manifest
                .get("total_stored_size")
                .and_then(Value::as_i64)
                .unwrap_or(total_len);
            if claimed_total != total_len {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            if let Some(header) = header {
                let header_representation_id = string_value(header, "representation_id", "");
                let manifest_representation_id = get_map(manifest, "representation")
                    .map(|m| string_value(m, "representation_id", ""))
                    .unwrap_or_default();
                if !header_representation_id.is_empty()
                    && !manifest_representation_id.is_empty()
                    && header_representation_id != manifest_representation_id
                {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                let stored_size = header
                    .get("stored_size")
                    .and_then(Value::as_i64)
                    .unwrap_or(claimed_total);
                if stored_size != claimed_total {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
            }
        }
        return accept(
            mapping([("validation_level", ystr("descriptor_consistent"))]),
            empty_map(),
        );
    }
    let may_decrypt = get_map(local_state, "access_state")
        .and_then(|m| m.get("may_decrypt"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(obj) = get_map(semantic_input, "object") {
        let realized = must_hex_to_bytes(&string_value(obj, "realized_bytes_hex", "0x"));
        let mut payload = b"edgerun:v0:object\0raw-bytes-v0\0".to_vec();
        payload.extend_from_slice(&realized);
        let object_id = bytes_to_hex(&sha256(&payload));
        let mut claimed = string_value(obj, "object_id", "");
        if claimed.is_empty() {
            claimed = get_map(obj, "descriptor")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
        }
        if !claimed.is_empty() && claimed != object_id {
            return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
        }
        let canonicalization_id = get_map(obj, "descriptor")
            .map(|m| string_value(m, "canonicalization_id", "raw-bytes-v0"))
            .unwrap_or_else(|| "raw-bytes-v0".to_string());
        return accept(
            mapping([
                ("validation_level", ystr("logical_object_valid")),
                ("object_id", ystr(object_id)),
                ("canonicalization_id", ystr(canonicalization_id)),
            ]),
            empty_map(),
        );
    }
    if let Some(rep) = get_map(semantic_input, "representation") {
        let stored = if let Some(manifest) =
            get_map(rep, "chunk_manifest").or_else(|| get_map(semantic_input, "chunk_manifest"))
        {
            let entries = get_seq(manifest, "entries").unwrap_or(&[]);
            let available = local_state.get("available_chunks").and_then(Value::as_map);
            let mut assembled = Vec::new();
            for entry in entries {
                let Some(entry) = entry.as_map() else {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                };
                let digest_hex = string_value(entry, "chunk_digest", "");
                let Some(available) = available else {
                    return defer(
                        ReasonCode::MissingDependency,
                        mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                    );
                };
                let Some(chunk_value) = available.get(&digest_hex).and_then(Value::as_str) else {
                    return defer(
                        ReasonCode::MissingDependency,
                        mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                    );
                };
                let chunk_bytes = must_hex_to_bytes(chunk_value);
                let mut chunk_payload = b"edgerun:v0:chunk-bytes\0".to_vec();
                chunk_payload.extend_from_slice(&chunk_bytes);
                let actual_digest = bytes_to_hex(&sha256(&chunk_payload));
                if !digest_hex.is_empty() && actual_digest != digest_hex {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                assembled.extend_from_slice(&chunk_bytes);
            }
            assembled
        } else {
            must_hex_to_bytes(&string_value(rep, "stored_bytes_hex", "0x"))
        };
        let mut payload = b"edgerun:v0:representation-bytes\0".to_vec();
        payload.extend_from_slice(&stored);
        let digest = bytes_to_hex(&sha256(&payload));
        let header = get_map(rep, "header");
        let claimed = if string_value(rep, "representation_digest", "").is_empty() {
            header
                .map(|m| string_value(m, "representation_digest", ""))
                .unwrap_or_default()
        } else {
            string_value(rep, "representation_digest", "")
        };
        let encryption_scheme = header
            .and_then(|m| m.get("encryption_scheme"))
            .cloned()
            .unwrap_or(Value::Null);
        if !claimed.is_empty() && claimed != digest {
            return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
        }
        let require_logical_object = semantic_input
            .get("require_logical_object")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if require_logical_object && !may_decrypt {
            return defer(
                ReasonCode::MissingDependency,
                mapping([
                    ("validation_level", ystr("deferred_missing_access")),
                    ("representation_digest", ystr(digest)),
                ]),
            );
        }
        let level = if may_decrypt {
            "logical_object_valid"
        } else {
            "representation_valid_only"
        };
        return accept(
            mapping([
                ("validation_level", ystr(level)),
                ("representation_digest", ystr(digest)),
                ("encryption_scheme", encryption_scheme),
            ]),
            empty_map(),
        );
    }
    reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map())
}

