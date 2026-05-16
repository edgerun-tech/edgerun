#![cfg(feature = "std")]

use crate::ssh_support::SshTarget;
use edgerun_crypto::{Ed25519SigningKey as SigningKey, fill_random};
use edgerun_protocols::seal::{SealKey, seal_with_key, unseal_with_key};
use edgerun_protocols::wire as edgerun_wire;
use edgerun_protocols::wire::{
    CapabilityResponseProofRecord, SdkWireRecord, SigningAlgorithmRecord,
    StorageWriteReceiptRecord, UserProfileIdSeedRecord, sdk_wire_bytes,
};
use edgerun_sdk::{
    runtime_api, ApiFunction, ChainManifest, CompositionComponent, CompositionManifest, Determinism,
    SDK_ABI_NAME, SegmentManifest, UnitManifest, sha256, sha256_hex, str_eq,
};
use formats::{
    bytes_to_hex, parse_api, parse_chain, parse_composition, parse_report, parse_segment,
    parse_segment_report,
};
use std::env;
use std::ffi::{CStr, CString};
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
mod formats;

mod deploy;
mod deploy_support;
mod marketplace;
mod package;
mod runtime;
mod ssh_support;

pub(crate) use deploy::*;
pub(crate) use marketplace::*;
pub(crate) use package::*;
pub(crate) use runtime::*;

fn main() {
    let mut args = env::args().skip(1);
    let code = match args.next().as_deref() {
        Some("list") => cmd_list(),
        Some("verify") => cmd_verify(args.next().as_deref()),
        Some("verify-chain") => cmd_verify_chain(args.next().as_deref()),
        Some("verify-segment") => cmd_verify_segment(args.next().as_deref()),
        Some("explain") => cmd_explain(args.next().as_deref()),
        Some("run-segment") => cmd_run_segment(args.collect()),
        Some("run-composition") => cmd_run_composition(args.collect()),
        Some("quote-composition") => cmd_quote_composition(args.collect()),
        Some("preflight-composition") => cmd_preflight_composition(args.collect()),
        Some("verify-report") => cmd_verify_report(args.next().as_deref()),
        Some("verify-segment-report") => cmd_verify_segment_report(args.next().as_deref()),
        Some("replay-segment-report") => cmd_replay_segment_report(args.collect()),
        Some("sign-segment-report") => cmd_sign_segment_report(args.collect()),
        Some("write-signer-policy") => cmd_write_signer_policy(args.collect()),
        Some("verify-signed-segment-report") => cmd_verify_signed_segment_report(args.collect()),
        Some("verify-chain-reports") => cmd_verify_chain_reports(args.collect()),
        Some("bench-unit") => cmd_bench_unit(args.collect()),
        Some("package-app") => cmd_package_app(args.collect()),
        Some("sign-app") => cmd_sign_app(args.collect()),
        Some("verify-signed-app") => cmd_verify_signed_app(args.collect()),
        Some("write-app-store-catalog") => cmd_write_app_store_catalog(args.collect()),
        Some("issue-product") => cmd_issue_product(args.collect()),
        Some("verify-product") => cmd_verify_product(args.collect()),
        Some("issue-entitlement") => cmd_issue_entitlement(args.collect()),
        Some("verify-entitlement") => cmd_verify_entitlement(args.collect()),
        Some("issue-settlement") => cmd_issue_settlement(args.collect()),
        Some("verify-settlement") => cmd_verify_settlement(args.collect()),
        Some("issue-payment-intent") => cmd_issue_payment_intent(args.collect()),
        Some("settle-payment") => cmd_settle_payment(args.collect()),
        Some("verify-payment") => cmd_verify_payment(args.collect()),
        Some("write-trust-policy") => cmd_write_trust_policy(args.collect()),
        Some("verify-trusted-app") => cmd_verify_trusted_app(args.collect()),
        Some("verify-trusted-product") => cmd_verify_trusted_product(args.collect()),
        Some("verify-trusted-entitlement") => cmd_verify_trusted_entitlement(args.collect()),
        Some("verify-trusted-payment") => cmd_verify_trusted_payment(args.collect()),
        Some("verify-trusted-settlement") => cmd_verify_trusted_settlement(args.collect()),
        Some("write-revocation") => cmd_write_revocation(args.collect()),
        Some("verify-revocation") => cmd_verify_revocation(args.collect()),
        Some("write-capability-request") => cmd_write_capability_request(args.collect()),
        Some("verify-capability-response") => cmd_verify_capability_response(args.collect()),
        Some("write-sign-request") => cmd_write_sign_request(args.collect()),
        Some("sign-request") => cmd_sign_request(args.collect()),
        Some("sign-request-authorized") => cmd_sign_request_authorized(args.collect()),
        Some("verify-sign-response") => cmd_verify_sign_response(args.collect()),
        Some("write-seal-request") => cmd_write_seal_request(args.collect()),
        Some("seal-request") => cmd_seal_request(args.collect()),
        Some("seal-request-authorized") => cmd_seal_request_authorized(args.collect()),
        Some("write-unseal-request") => cmd_write_unseal_request(args.collect()),
        Some("unseal-request") => cmd_unseal_request(args.collect()),
        Some("unseal-request-authorized") => cmd_unseal_request_authorized(args.collect()),
        Some("verify-seal-response") => cmd_verify_seal_response(args.collect()),
        Some("write-storage-read-request") => cmd_write_storage_read_request(args.collect()),
        Some("write-storage-write-request") => cmd_write_storage_write_request(args.collect()),
        Some("storage-read-request") => cmd_storage_read_request(args.collect()),
        Some("storage-read-request-authorized") => {
            cmd_storage_read_request_authorized(args.collect())
        }
        Some("storage-write-request") => cmd_storage_write_request(args.collect()),
        Some("storage-write-request-authorized") => {
            cmd_storage_write_request_authorized(args.collect())
        }
        Some("verify-storage-response") => cmd_verify_storage_response(args.collect()),
        Some("create-user-profile") => cmd_create_user_profile(args.collect()),
        Some("grant-profile-capability") => cmd_grant_profile_capability(args.collect()),
        Some("open-user-profile") => cmd_open_user_profile(args.collect()),
        Some("verify-profile-access") => cmd_verify_profile_access(args.collect()),
        Some("create-user-profile-password") => cmd_create_user_profile_password(args.collect()),
        Some("open-user-profile-password") => cmd_open_user_profile_password(args.collect()),
        Some("generate-unit-metadata") => cmd_generate_unit_metadata(args.next().as_deref()),
        Some("deploy-inventory") => cmd_deploy_inventory(args.collect()),
        Some("deploy-server") => cmd_deploy_server(args.collect()),
        Some("deploy-ssh-probe") => cmd_deploy_ssh_probe(args.collect()),
        Some("deploy-ssh-exec") => cmd_deploy_ssh_exec(args.collect()),
        Some("build-artifacts") => cmd_build_artifacts(),
        _ => {
            eprintln!(
                "usage: edgerun-sdk <...>|write-revocation <out.erev> <kind> <target-hex> <issuer-seed-hex> <issued-at> [reason]|verify-revocation <revocation.erev>|verify-trusted-* ... [revocation.erev]"
            );
            1
        }
    };
    std::process::exit(code);
}
