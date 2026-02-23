#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd curl
require_cmd jq

if [[ -z "${CF_ACCOUNT_ID:-}" ]]; then
  echo "CF_ACCOUNT_ID is required" >&2
  exit 1
fi

if [[ -z "${CF_API_TOKEN:-}" ]]; then
  echo "CF_API_TOKEN is required" >&2
  echo "Token must have Zero Trust Access app/policy write permissions." >&2
  exit 1
fi

ACCESS_HOSTNAME="${EDGERUN_TERM_ACCESS_HOSTNAME:-term.edgerun.tech}"
SESSION_DURATION="${EDGERUN_TERM_ACCESS_SESSION_DURATION:-24h}"
ALLOW_EMAIL="${EDGERUN_TERM_ACCESS_ALLOW_EMAIL:-}"
ALLOW_DOMAIN="${EDGERUN_TERM_ACCESS_ALLOW_DOMAIN:-edgerun.tech}"
APP_NAME="${EDGERUN_TERM_ACCESS_APP_NAME:-edgerun-term}"
POLICY_NAME="${EDGERUN_TERM_ACCESS_POLICY_NAME:-allow-edgerun-term}"

api() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  if [[ -n "$body" ]]; then
    curl -sS -X "$method" \
      -H "Authorization: Bearer ${CF_API_TOKEN}" \
      -H "Content-Type: application/json" \
      "https://api.cloudflare.com/client/v4${path}" \
      --data "$body"
  else
    curl -sS -X "$method" \
      -H "Authorization: Bearer ${CF_API_TOKEN}" \
      "https://api.cloudflare.com/client/v4${path}"
  fi
}

fail_if_error() {
  local payload="$1"
  if [[ "$(jq -r '.success // false' <<<"$payload")" != "true" ]]; then
    echo "$payload" | jq .
    echo "Cloudflare API call failed" >&2
    exit 1
  fi
}

APP_LIST="$(api GET "/accounts/${CF_ACCOUNT_ID}/access/apps")"
fail_if_error "$APP_LIST"

APP_ID="$(
  jq -r --arg host "$ACCESS_HOSTNAME" '
    .result[]? | select((.domain // "") == $host) | .id
  ' <<<"$APP_LIST" | head -n1
)"

if [[ -z "$APP_ID" ]]; then
  APP_CREATE_BODY="$(
    jq -n \
      --arg name "$APP_NAME" \
      --arg domain "$ACCESS_HOSTNAME" \
      --arg duration "$SESSION_DURATION" \
      '{
        name: $name,
        type: "self_hosted",
        domain: $domain,
        session_duration: $duration,
        auto_redirect_to_identity: false
      }'
  )"
  APP_CREATE="$(api POST "/accounts/${CF_ACCOUNT_ID}/access/apps" "$APP_CREATE_BODY")"
  fail_if_error "$APP_CREATE"
  APP_ID="$(jq -r '.result.id' <<<"$APP_CREATE")"
  echo "Created Access app: ${APP_ID}"
else
  APP_PATCH_BODY="$(
    jq -n \
      --arg name "$APP_NAME" \
      --arg domain "$ACCESS_HOSTNAME" \
      --arg duration "$SESSION_DURATION" \
      '{
        name: $name,
        type: "self_hosted",
        domain: $domain,
        session_duration: $duration,
        auto_redirect_to_identity: false
      }'
  )"
  APP_PATCH="$(api PUT "/accounts/${CF_ACCOUNT_ID}/access/apps/${APP_ID}" "$APP_PATCH_BODY")"
  fail_if_error "$APP_PATCH"
  echo "Updated Access app: ${APP_ID}"
fi

if [[ -n "$ALLOW_EMAIL" ]]; then
  INCLUDE_EXPR="$(
    jq -n --arg email "$ALLOW_EMAIL" '[{email:{email:$email}}]'
  )"
else
  INCLUDE_EXPR="$(
    jq -n --arg domain "$ALLOW_DOMAIN" '[{email_domain:{domain:$domain}}]'
  )"
fi

POLICY_LIST="$(api GET "/accounts/${CF_ACCOUNT_ID}/access/apps/${APP_ID}/policies")"
fail_if_error "$POLICY_LIST"
POLICY_ID="$(
  jq -r --arg name "$POLICY_NAME" '
    .result[]? | select((.name // "") == $name) | .id
  ' <<<"$POLICY_LIST" | head -n1
)"

POLICY_BODY="$(
  jq -n \
    --arg name "$POLICY_NAME" \
    --argjson include "$INCLUDE_EXPR" \
    '{
      name: $name,
      decision: "allow",
      include: $include,
      require: [],
      exclude: []
    }'
)"

if [[ -z "$POLICY_ID" ]]; then
  POLICY_CREATE="$(api POST "/accounts/${CF_ACCOUNT_ID}/access/apps/${APP_ID}/policies" "$POLICY_BODY")"
  fail_if_error "$POLICY_CREATE"
  POLICY_ID="$(jq -r '.result.id' <<<"$POLICY_CREATE")"
  echo "Created Access policy: ${POLICY_ID}"
else
  POLICY_PATCH="$(api PUT "/accounts/${CF_ACCOUNT_ID}/access/apps/${APP_ID}/policies/${POLICY_ID}" "$POLICY_BODY")"
  fail_if_error "$POLICY_PATCH"
  echo "Updated Access policy: ${POLICY_ID}"
fi

echo "Access setup complete for https://${ACCESS_HOSTNAME}"
