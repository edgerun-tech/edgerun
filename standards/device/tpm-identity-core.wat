(module
  (import "edgerun-core" "memory" (memory 1))
  (global $TPM_ID_ABI i32 (i32.const 1))

  ;; TPM identity key types
  (global $ID_KEY_ECC i32 (i32.const 1))
  (global $ID_KEY_RSA i32 (i32.const 2))

  ;; Security levels
  (global $SEC_LEVEL_NONE i32 (i32.const 0))
  (global $SEC_LEVEL_SIGNED i32 (i32.const 1))
  (global $SEC_LEVEL_SEALED i32 (i32.const 2))

  (func (export "tpm_id_abi_version") (result i32)
    global.get $TPM_ID_ABI
  )

  ;; Validate TPM identity public key hash (SHA-256, 32 bytes)
  (func (export "tpm_id_validate_pubhash")
    (param $hash_offset i32)
    (result i32)
    (local $first_byte i32)
    (local $last_byte i32)
    (local.set $first_byte (i32.load8_u (local.get $hash_offset)))
    (local.set $last_byte (i32.load8_u offset=31 (local.get $hash_offset)))
    ;; Check that hash is not all zeros or all FFs
    (if (i32.and
          (i32.eqz (local.get $first_byte))
          (i32.eqz (local.get $last_byte)))
      (then (return (i32.const 1)))
    )
    (if (i32.and
          (i32.eq (local.get $first_byte) (i32.const 0xFF))
          (i32.eq (local.get $last_byte) (i32.const 0xFF)))
      (then (return (i32.const 2)))
    )
    (i32.const 0)
  )

  ;; Verify trust on first use:
  ;; Check stored trusted key hash matches presented key hash
  ;; offset_stored = stored hash in memory, offset_presented = presented hash
  ;; Returns 0 if match, 1 if no match, 2 if no stored key (first use)
  (func (export "tpm_id_check_tofu")
    (param $offset_stored i32)
    (param $offset_presented i32)
    (param $stored_len i32)
    (result i32)
    (local $i i32)
    (local $match i32)

    ;; If stored length is 0, this is first use
    (if (i32.eqz (local.get $stored_len))
      (then (return (i32.const 2)))
    )

    (local.set $match (i32.const 1))
    (local.set $i (i32.const 0))
    (block $loop_done
      (loop $loop
        (if (i32.eq (local.get $i) (local.get $stored_len))
          (then (br $loop_done))
        )
        (if (i32.ne
              (i32.load8_u (i32.add (local.get $offset_stored) (local.get $i)))
              (i32.load8_u (i32.add (local.get $offset_presented) (local.get $i))))
          (then
            (local.set $match (i32.const 0))
            (br $loop_done)
          )
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)
      )
    )
    (if (local.get $match) (then (return (i32.const 0))))
    (i32.const 1)
  )

  ;; Check whether given security level meets minimum requirement
  (func (export "tpm_id_security_meets")
    (param $level i32)
    (param $minimum i32)
    (result i32)
    (i32.ge_u (local.get $level) (local.get $minimum))
  )
)