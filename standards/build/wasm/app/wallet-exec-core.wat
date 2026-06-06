(module
  (import "edgerun-core" "memory" (memory 1))
;; Wallet settlement and exec-runner semantics plundered from edgerun-wallet and edgerun-exec-runner.

  (func (export "wallet_exec_abi_version") (result i32) i32.const 1)
  (func (export "wallet_core_abi_version") (result i32) i32.const 1)
  (func (export "exec_runner_version") (result i32) i32.const 1)
  (func (export "exec_runner_magic") (result i32) i32.const 0x52585245) ;; ERXR little-endian word
  (func (export "exec_max_program_size") (result i64) i64.const 536870912)
  (func (export "exec_max_blob_size") (result i64) i64.const 536870912)

  (func (export "wallet_chain_family_valid") (param $family i32) (result i32)
    ;; EdgeRun, BitcoinLike, EvmLike, SolanaLike, TronLike, Other.
    (if (i32.or (i32.and (i32.ge_u (local.get $family) (i32.const 1)) (i32.le_u (local.get $family) (i32.const 5))) (i32.eq (local.get $family) (i32.const 255))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "wallet_is_edgerun_chain") (param $family i32) (result i32)
    (if (i32.eq (local.get $family) (i32.const 1)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "wallet_amount_is_zero") (param $mantissa_hi i64) (param $mantissa_lo i64) (result i32)
    (if (i64.eqz (i64.or (local.get $mantissa_hi) (local.get $mantissa_lo))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "canonical_asset_id_parts") (param $has_contract i32) (result i32)
    ;; 2 parts: SYMBOL:network, 3 with contract.
    (if (local.get $has_contract) (then (return (i32.const 3))))
    i32.const 2)

  (func (export "wallet_event_apply_result")
    (param $event_kind i32) (param $amount_zero i32) (param $duplicate_claim i32)
    (param $duplicate_observation i32) (param $has_account i32) (param $sufficient_balance i32)
    (result i32)
    ;; event: 1 intent, 2 external observed, 3 emission claimed, 4 emission finalized, 5 burn claimed, 6 rejected.
    ;; 0 ok, 1 duplicate claim, 2 duplicate observation, 3 unknown account, 4 insufficient balance, 5 zero amount.
    (if (i32.eq (local.get $event_kind) (i32.const 2))
      (then
        (if (local.get $duplicate_observation) (then (return (i32.const 2))))
        (return (i32.const 0))))
    (if (i32.eq (local.get $event_kind) (i32.const 4))
      (then
        (if (local.get $amount_zero) (then (return (i32.const 5))))
        (if (local.get $duplicate_claim) (then (return (i32.const 1))))
        (return (i32.const 0))))
    (if (i32.eq (local.get $event_kind) (i32.const 5))
      (then
        (if (local.get $amount_zero) (then (return (i32.const 5))))
        (if (i32.eqz (local.get $has_account)) (then (return (i32.const 3))))
        (if (i32.eqz (local.get $sufficient_balance)) (then (return (i32.const 4))))
        (return (i32.const 0))))
    i32.const 0)

  (func (export "wallet_credit_result") (param $amount_zero i32) (param $overflow i32) (result i32)
    ;; 0 ok, 5 zero amount, 6 overflow.
    (if (local.get $amount_zero) (then (return (i32.const 5))))
    (if (local.get $overflow) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "wallet_debit_result") (param $amount_zero i32) (param $has_account i32) (param $sufficient_balance i32) (result i32)
    ;; 0 ok, 3 unknown account, 4 insufficient balance, 5 zero amount.
    (if (local.get $amount_zero) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $has_account)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $sufficient_balance)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "wallet_emission_evidence_result")
    (param $amount_zero i32) (param $admission_ok i32) (param $receipt_ok i32) (param $notary_ok i32)
    (param $claim_hashes_match i32) (param $receipt_binds_admission i32) (param $notary_binds_admission i32)
    (result i32)
    ;; 0 ok, 1 invalid admission, 2 invalid receipt, 3 invalid notary, 4 proof mismatch, 5 zero amount.
    (if (local.get $amount_zero) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $admission_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $receipt_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $notary_ok)) (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (i32.and (local.get $claim_hashes_match) (local.get $receipt_binds_admission)) (local.get $notary_binds_admission))) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "wallet_preimage_domain_code") (param $kind i32) (result i32)
    ;; 1 chain, 2 asset, 3 account, 4 address, 5 transfer, 6 observation, 7 emission claim.
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 7))) (then (return (local.get $kind))))
    i32.const 0)

  (func (export "exec_msg_valid") (param $msg i32) (result i32)
    ;; common 1,10..14, legacy exec 2..4, CAS 20..25.
    (if (i32.eq (local.get $msg) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $msg) (i32.const 2)) (i32.le_u (local.get $msg) (i32.const 4))) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $msg) (i32.const 10)) (i32.le_u (local.get $msg) (i32.const 14))) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $msg) (i32.const 20)) (i32.le_u (local.get $msg) (i32.const 25))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exec_read_frame_result") (param $magic_ok i32) (param $version i32) (result i32)
    ;; 0 ok, 1 bad magic, 2 bad version.
    (if (i32.eqz (local.get $magic_ok)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $version) (i32.const 1)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "exec_send_frame_result") (param $payload_len_hi i32) (result i32)
    ;; payload must fit u32.
    (if (i32.ne (local.get $payload_len_hi) (i32.const 0)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exec_begin_result") (param $payload_len i32) (param $size i64) (param $argc i32) (result i32)
    ;; 0 ok, 1 truncated fixed fields, 2 program too large. Empty argv defaults to ./program.
    (if (i32.lt_u (local.get $payload_len) (i32.const 84)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $size) (i64.const 536870912)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "exec_argv_count_after_parse") (param $argc i32) (result i32)
    (if (i32.eqz (local.get $argc)) (then (return (i32.const 1))))
    local.get $argc)

  (func (export "exec_job_chunk_result") (param $msg_type i32) (param $chunk_len i64) (param $remaining i64) (result i32)
    ;; 0 ok, 1 expected chunk, 2 chunk exceeds declared size.
    (if (i32.ne (local.get $msg_type) (i32.const 3)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $chunk_len) (local.get $remaining)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "exec_job_end_result") (param $msg_type i32) (result i32)
    ;; 0 ok, 1 expected EXEC_END.
    (if (i32.ne (local.get $msg_type) (i32.const 4)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exec_exit_code") (param $status_has_code i32) (param $code i32) (param $signal i32) (param $timed_out i32) (result i32)
    (if (local.get $timed_out) (then (return (i32.const 124))))
    (if (local.get $status_has_code) (then (return (local.get $code))))
    (i32.add (i32.const 128) (local.get $signal)))

  (func (export "exec_pipe_msg_type") (param $is_stderr i32) (result i32)
    (if (local.get $is_stderr) (then (return (i32.const 11))))
    i32.const 10)

  (func (export "cas_root_kind") (param $kind i32) (result i32)
    ;; 1 blobs/sha256, 2 jobs.
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 2))) (then (return (local.get $kind))))
    i32.const 0)

  (func (export "cas_put_begin_result") (param $payload_len i32) (param $size i64) (result i32)
    ;; 0 ok, 1 truncated fixed fields, 2 blob too large.
    (if (i32.lt_u (local.get $payload_len) (i32.const 44)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $size) (i64.const 536870912)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "cas_put_result") (param $blob_exists i32) (param $hash_matches i32) (result i32)
    ;; 23 PUT_OK, 25 BLOB_EXISTS, 13 ERROR.
    (if (local.get $blob_exists) (then (return (i32.const 25))))
    (if (i32.eqz (local.get $hash_matches)) (then (return (i32.const 13))))
    i32.const 23)

  (func (export "cas_put_chunk_result") (param $msg_type i32) (param $chunk_len i64) (param $remaining i64) (result i32)
    ;; 0 ok, 1 expected PUT_CHUNK, 2 chunk exceeds declared size.
    (if (i32.ne (local.get $msg_type) (i32.const 21)) (then (return (i32.const 1))))
    (if (i64.gt_u (local.get $chunk_len) (local.get $remaining)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "cas_put_end_result") (param $msg_type i32) (result i32)
    (if (i32.ne (local.get $msg_type) (i32.const 22)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "cas_exec_result") (param $blob_exists i32) (result i32)
    ;; 0 execute, 13 send error.
    (if (i32.eqz (local.get $blob_exists)) (then (return (i32.const 13))))
    i32.const 0)

  (func (export "hex32_result") (param $len i32) (param $all_hex i32) (result i32)
    ;; 0 ok, 1 wrong length, 2 invalid hex.
    (if (i32.ne (local.get $len) (i32.const 64)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $all_hex)) (then (return (i32.const 2))))
    i32.const 0)
)