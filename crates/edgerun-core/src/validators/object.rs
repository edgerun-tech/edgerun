use crate::prelude::v1::*;

use super::helpers::*;
pub fn validate_object_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
) -> ValidationResult {
    if let Some(descriptor) = get_map(semantic_input, "descriptor") {
        let header = get_map(semantic_input, "header");
        let manifest = get_map(semantic_input, "chunk_manifest");
        let descriptor_object_id = string_value(descriptor, "object_id", "");
        if descriptor_object_id.is_empty()
            || number_value(descriptor, "descriptor_version", 0) != 1
            || matches!(
                descriptor.get("object_kind").and_then(Value::as_str),
                None | Some("") | Some("OBJECT_KIND_UNSPECIFIED")
            )
            || number_value(descriptor, "object_schema_version", 0) <= 0
            || string_value(descriptor, "canonicalization_id", "").is_empty()
            || string_value(descriptor, "canonical_digest", "").is_empty()
            || !descriptor.contains_key("canonical_size")
            || number_value(descriptor, "canonical_size", -1) < 0
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if !object_ref_is_valid(descriptor.get("describes_object"))
            || !object_ref_is_valid(descriptor.get("object_metadata"))
        {
            return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
        }
        if let Some(header) = header {
            if let Some(reason) = version_field_error(header, "header_version") {
                return reject(reason, empty_map(), empty_map());
            }
            if string_value(header, "representation_id", "").is_empty() {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            let header_object = get_map(header, "object")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
            if header_object.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if !object_ref_is_valid(header.get("chunk_manifest_object"))
                || !object_ref_is_valid(header.get("access_package_object"))
                || !object_ref_is_valid(header.get("representation_metadata"))
            {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if !descriptor_object_id.is_empty()
                && !header_object.is_empty()
                && descriptor_object_id != header_object
            {
                return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
            }
            if !digest_value_is_present(header.get("representation_digest")) {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            let chunking_mode = string_value(header, "chunking_mode", "");
            if chunking_mode.is_empty() || chunking_mode == "CHUNKING_MODE_UNSPECIFIED" {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            let stored_size = number_value(header, "stored_size", -1);
            if stored_size <= 0 {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    mapping([("reason", ystr("stored_size_zero_or_negative"))]),
                    empty_map(),
                );
            }
            if chunking_mode == "CHUNKING_MODE_MANIFEST" && manifest.is_none() {
                return defer(
                    ReasonCode::MissingDependency,
                    mapping([("validation_level", ystr("deferred_missing_chunks"))]),
                );
            }
        }
        if let Some(manifest) = manifest {
            if let Some(reason) = version_field_error(manifest, "manifest_version") {
                return reject(reason, empty_map(), empty_map());
            }
            let manifest_object = get_map(manifest, "object")
                .map(|m| string_value(m, "object_id", ""))
                .unwrap_or_default();
            if manifest_object.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if !object_ref_is_valid(manifest.get("manifest_metadata")) {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            if !descriptor_object_id.is_empty()
                && !manifest_object.is_empty()
                && descriptor_object_id != manifest_object
            {
                return reject(ReasonCode::ObjectIdMismatch, empty_map(), empty_map());
            }
            let entries = get_seq(manifest, "entries")
                .or_else(|| get_seq(manifest, "chunk_entries"))
                .unwrap_or(&[]);
            if entries.is_empty() {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            if !manifest.contains_key("chunk_count")
                || !manifest.contains_key("total_stored_size")
                || number_value(manifest, "chunk_count", 0) <= 0
                || number_value(manifest, "total_stored_size", 0) <= 0
            {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            let claimed_count = manifest
                .get("chunk_count")
                .and_then(Value::as_i64)
                .unwrap_or(entries.len() as i64);
            if claimed_count != entries.len() as i64 {
                return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
            }
            for (index, entry) in entries.iter().enumerate() {
                let Some(entry) = entry.as_map() else {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                };
                if let Some(Value::String(representation_id)) = entry.get("representation_id") {
                    if representation_id.is_empty() {
                        return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                    }
                }
                if let Some(Value::String(digest)) = entry.get("chunk_digest") {
                    if digest.is_empty() {
                        return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                    }
                } else if !digest_value_is_present(entry.get("chunk_digest")) {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                if !entry.contains_key("length") || number_value(entry, "length", -1) <= 0 {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                if !entry.contains_key("offset") || number_value(entry, "offset", -1) < 0 {
                    return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                }
                if let Some(entry_index) = entry.get("index").and_then(Value::as_i64) {
                    if entry_index != index as i64 {
                        return reject(ReasonCode::RepresentationInvalid, empty_map(), empty_map());
                    }
                }
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
                // Cross-check representation IDs between header and manifest
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
                // Spec §14.17: stored_size is required — proto3 uint64 defaults to 0,
                // so explicitly reject stored_size == 0 as invalid (empty representation)
                let stored_size = header
                    .get("stored_size")
                    .and_then(Value::as_i64)
                    .unwrap_or(claimed_total);
                if stored_size <= 0 {
                    return reject(
                        ReasonCode::RepresentationInvalid,
                        mapping([("reason", ystr("stored_size_zero_or_negative"))]),
                        empty_map(),
                    );
                }
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

fn object_ref_is_valid(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(object_id)) => !object_id.is_empty(),
        Some(Value::Map(map)) => map
            .get("object_id")
            .and_then(Value::as_str)
            .is_some_and(|object_id| !object_id.is_empty()),
        Some(_) => false,
    }
}

fn digest_value_is_present(value: Option<&Value>) -> bool {
    match value {
        Some(Value::String(digest)) => !digest.is_empty(),
        Some(Value::Map(map)) => map
            .get("value")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty()),
        _ => false,
    }
}
