#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  $0 --type ecc-p256|ecc-p384|rsa-2048 --handle 0x81010020 --out-dir ./var/tpm-key [--hierarchy o|e|p] [--auth secret] [--policy ./var/tpm-policy/policy.digest] [--fixture node_server]

Creates a TPM key, persists it at the requested handle, exports a PEM public key,
and writes daemon-ready environment files and a launch helper.
Outputs:
  <out-dir>/primary.ctx
  <out-dir>/key.pub
  <out-dir>/key.priv
  <out-dir>/key.ctx
  <out-dir>/public.pem
  <out-dir>/metadata.env
  <out-dir>/edgerund.env
  <out-dir>/run-edgerund.sh
USAGE
}

type=""
handle=""
out_dir=""
hierarchy="o"
auth=""
policy_file=""
fixture="node_server"
repo_root="$(pwd)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --type) type="$2"; shift 2 ;;
    --handle) handle="$2"; shift 2 ;;
    --out-dir) out_dir="$2"; shift 2 ;;
    --hierarchy) hierarchy="$2"; shift 2 ;;
    --auth) auth="$2"; shift 2 ;;
    --policy) policy_file="$2"; shift 2 ;;
    --fixture) fixture="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$type" && -n "$handle" && -n "$out_dir" ]] || { usage; exit 1; }
[[ -z "$policy_file" || -f "$policy_file" ]] || { echo "policy file not found: $policy_file" >&2; exit 1; }
mkdir -p "$out_dir"

primary="$out_dir/primary.ctx"
pub="$out_dir/key.pub"
priv="$out_dir/key.priv"
ctx="$out_dir/key.ctx"
pem="$out_dir/public.pem"
meta="$out_dir/metadata.env"
daemon_env="$out_dir/edgerund.env"
run_helper="$out_dir/run-edgerund.sh"

case "$type" in
  ecc-p256)
    create_args=(-G ecc -g sha256 -a "sign|fixedtpm|fixedparent|sensitivedataorigin|userwithauth" -u "$pub" -r "$priv")
    sig_alg=ecdsa-p256
    ;;
  ecc-p384)
    create_args=(-G ecc384 -g sha384 -a "sign|fixedtpm|fixedparent|sensitivedataorigin|userwithauth" -u "$pub" -r "$priv")
    sig_alg=ecdsa-p384
    ;;
  rsa-2048)
    create_args=(-G rsa2048 -g sha256 -a "sign|fixedtpm|fixedparent|sensitivedataorigin|userwithauth" -u "$pub" -r "$priv")
    sig_alg=rsa-pss-sha256
    ;;
  *) echo "unsupported type: $type" >&2; exit 1 ;;
esac

if [[ -n "$policy_file" ]]; then
  create_args+=(-L "$policy_file")
fi

tpm2_createprimary -Q -C "$hierarchy" -c "$primary"
if [[ -n "$auth" ]]; then
  tpm2_create -Q -C "$primary" -p "$auth" "${create_args[@]}"
  tpm2_load -Q -C "$primary" -u "$pub" -r "$priv" -c "$ctx"
else
  tpm2_create -Q -C "$primary" "${create_args[@]}"
  tpm2_load -Q -C "$primary" -u "$pub" -r "$priv" -c "$ctx"
fi

tpm2_evictcontrol -Q -C "$hierarchy" -c "$ctx" "$handle"
tpm2_readpublic -Q -c "$handle" -f pem -o "$pem"

cat > "$meta" <<META
edgerunD_TPM_KEY_CONTEXT=$handle
edgerunD_TPM_PUBLIC_KEY=$pem
edgerunD_SIGNATURE_ALGORITHM=$sig_alg
META
if [[ -n "$auth" ]]; then
  printf 'edgerunD_TPM_KEY_AUTH=%s\n' "$auth" >> "$meta"
fi
if [[ -n "$policy_file" ]]; then
  printf 'edgerunD_TPM_POLICY=%s\n' "$policy_file" >> "$meta"
fi

cat > "$daemon_env" <<META
export edgerunD_TPM_KEY_CONTEXT=$handle
export edgerunD_TPM_PUBLIC_KEY=$pem
export edgerunD_SIGNATURE_ALGORITHM=$sig_alg
export edgerunD_FIXTURE=$(printf '%q' "$fixture")
META
if [[ -n "$auth" ]]; then
  printf 'export edgerunD_TPM_KEY_AUTH=%q\n' "$auth" >> "$daemon_env"
fi
if [[ -n "$policy_file" ]]; then
  printf 'export edgerunD_TPM_POLICY=%q\n' "$policy_file" >> "$daemon_env"
fi

cat > "$run_helper" <<META
#!/usr/bin/env bash
set -euo pipefail
script_dir="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
cd $(printf '%q' "$repo_root")/rust
source "\$script_dir/$(basename "$daemon_env")"
args=(cargo run --release -p edgerun-node --bin edgerund --
  --fixture "\${edgerunD_FIXTURE:-$fixture}"
  --tpm-key-context "\${edgerunD_TPM_KEY_CONTEXT}"
  --tpm-public-key "\${edgerunD_TPM_PUBLIC_KEY}"
  --signature-algorithm "\${edgerunD_SIGNATURE_ALGORITHM}")
if [[ -n "\${edgerunD_TPM_KEY_AUTH:-}" ]]; then
  args+=(--tpm-key-auth "\${edgerunD_TPM_KEY_AUTH}")
fi
exec "\${args[@]}" "\$@"
META
chmod +x "$run_helper"

printf 'Provisioned %s at %s\nPublic key: %s\nMetadata: %s\nDaemon env: %s\nRun helper: %s\n' "$type" "$handle" "$pem" "$meta" "$daemon_env" "$run_helper"
