#!/bin/sh
set -eu

# Build web-facing Edgerun binaries on the developer machine and deploy only
# completed artifacts to the small production server.
#
# Defaults match the current single-server setup:
#   scripts/release-web-surfaces.sh
#
# Useful overrides:
#   scripts/release-web-surfaces.sh --host mail --branch main --ref HEAD
#   SKIP_PUSH=1 scripts/release-web-surfaces.sh

HOST="mail"
BRANCH="main"
REF="HEAD"
REMOTE="origin"
REMOTE_REPO="/srv/git/edgerun_core.git"
REMOTE_CHECKOUT="/srv/edgerun_core"
REMOTE_BLOG_OUT="/srv/blog/.generated"
DASH_MODULES_ROOT="/srv/dash/modules"
SERVICE="edgerun-server"
BLOG_TITLE="EdgeRun Build Log"
BLOG_DESCRIPTION="Feature-by-feature notes on building Edgerun from its source tree."
BLOG_BASE_URL="https://blog.edgerun.tech"
DASH_HOST="dash.edgerun.tech"
BLOG_HOST="blog.edgerun.tech"
GIT_HOST="git.edgerun.tech"
GIT_TITLE="Edgerun Git"
GIT_BASE_URL="https://git.edgerun.tech"

usage() {
    cat <<'USAGE'
usage: scripts/release-web-surfaces.sh [options]

Builds edgerun-server, edgerun-blog, and edgerun-git locally from a clean
temporary worktree, copies the binaries to the server, updates the server
checkout, regenerates static blog output, and restarts edgerun-server.

Options:
  --host HOST              ssh host to deploy to (default: mail)
  --branch BRANCH          branch to push/fetch/reset on the server (default: main)
  --ref REF                local git ref to build from (default: HEAD)
  --remote REMOTE          local git remote to push to (default: origin)
  --remote-repo PATH       bare repo path on server (default: /srv/git/edgerun_core.git)
  --checkout PATH          server checkout path (default: /srv/edgerun_core)
  --blog-out PATH          generated blog output path (default: /srv/blog/.generated)
  --dash-modules-root PATH dashboard Wasm module path (default: /srv/dash/modules)
  --service NAME           systemd service name (default: edgerun-server)
  --dash-host HOST         dashboard host (default: dash.edgerun.tech)
  --skip-push              do not push before deploying
  -h, --help               show this help

Environment:
  SKIP_PUSH=1              same as --skip-push
  CARGO_TARGET_DIR=PATH    reuse a specific target directory
USAGE
}

SKIP_PUSH="${SKIP_PUSH:-0}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --host)
            HOST="$2"
            shift 2
            ;;
        --branch)
            BRANCH="$2"
            shift 2
            ;;
        --ref)
            REF="$2"
            shift 2
            ;;
        --remote)
            REMOTE="$2"
            shift 2
            ;;
        --remote-repo)
            REMOTE_REPO="$2"
            shift 2
            ;;
        --checkout)
            REMOTE_CHECKOUT="$2"
            shift 2
            ;;
        --blog-out)
            REMOTE_BLOG_OUT="$2"
            shift 2
            ;;
        --dash-modules-root)
            DASH_MODULES_ROOT="$2"
            shift 2
            ;;
        --service)
            SERVICE="$2"
            shift 2
            ;;
        --dash-host)
            DASH_HOST="$2"
            shift 2
            ;;
        --skip-push)
            SKIP_PUSH=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf 'unknown argument: %s\n\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'missing required command: %s\n' "$1" >&2
        exit 1
    fi
}

need cargo
need git
need scp
need ssh

ROOT="$(git rev-parse --show-toplevel)"
COMMIT="$(git -C "$ROOT" rev-parse "$REF")"
SHORT_COMMIT="$(git -C "$ROOT" rev-parse --short "$COMMIT")"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/release-web-surfaces}"
WORKTREE="$(mktemp -d "${TMPDIR:-/tmp}/edgerun-release.XXXXXX")"

cleanup() {
    git -C "$ROOT" worktree remove --force "$WORKTREE" >/dev/null 2>&1 || rm -rf "$WORKTREE"
}
trap cleanup EXIT INT TERM

printf 'release ref: %s (%s)\n' "$REF" "$SHORT_COMMIT"
printf 'build host: local developer machine\n'
printf 'deploy host: %s\n' "$HOST"

git -C "$ROOT" worktree add --detach "$WORKTREE" "$COMMIT" >/dev/null

if [ "$SKIP_PUSH" != "1" ]; then
    printf 'pushing %s to %s/%s\n' "$SHORT_COMMIT" "$REMOTE" "$BRANCH"
    git -C "$ROOT" push "$REMOTE" "$COMMIT:refs/heads/$BRANCH"
else
    printf 'skipping push; server must already have %s on %s\n' "$SHORT_COMMIT" "$BRANCH"
fi

printf 'building release binaries locally\n'
(
    cd "$WORKTREE"
    CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p edgerun-server --bin edgerun-server --features std,smtp,imap,dns,tls
    CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p edgerun-blog --bin edgerun-blog
    CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p edgerun-git --bin edgerun-git
    CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p edgerun-dash-apps --target wasm32-unknown-unknown
)

binary_path() {
    name="$1"
    for candidate in \
        "$TARGET_DIR/x86_64-unknown-linux-musl/release/$name" \
        "$TARGET_DIR/release/$name"
    do
        if [ -x "$candidate" ]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    printf 'could not find built binary: %s\n' "$name" >&2
    exit 1
}

SERVER_BIN="$(binary_path edgerun-server)"
BLOG_BIN="$(binary_path edgerun-blog)"
GIT_BIN="$(binary_path edgerun-git)"
DASH_APPS_WASM="$TARGET_DIR/wasm32-unknown-unknown/release/edgerun_dash_apps.wasm"
if [ ! -f "$DASH_APPS_WASM" ]; then
    printf 'could not find built wasm module: %s\n' "$DASH_APPS_WASM" >&2
    exit 1
fi

printf 'copying built artifacts to %s:/tmp\n' "$HOST"
scp "$SERVER_BIN" "$BLOG_BIN" "$GIT_BIN" "$DASH_APPS_WASM" "$HOST:/tmp/"

printf 'installing and restarting on %s without remote compilation\n' "$HOST"
ssh "$HOST" \
    "BRANCH='$BRANCH' REMOTE_REPO='$REMOTE_REPO' REMOTE_CHECKOUT='$REMOTE_CHECKOUT' REMOTE_BLOG_OUT='$REMOTE_BLOG_OUT' DASH_MODULES_ROOT='$DASH_MODULES_ROOT' SERVICE='$SERVICE' BLOG_TITLE='$BLOG_TITLE' BLOG_DESCRIPTION='$BLOG_DESCRIPTION' BLOG_BASE_URL='$BLOG_BASE_URL' BLOG_HOST='$BLOG_HOST' GIT_HOST='$GIT_HOST' GIT_TITLE='$GIT_TITLE' GIT_BASE_URL='$GIT_BASE_URL' DASH_HOST='$DASH_HOST' sh -s" <<'REMOTE'
set -eu
sudo install -m 0755 /tmp/edgerun-server /usr/local/bin/edgerun-server
sudo install -m 0755 /tmp/edgerun-blog /usr/local/bin/edgerun-blog
sudo install -m 0755 /tmp/edgerun-git /usr/local/bin/edgerun-git
sudo install -d -m 0755 "$DASH_MODULES_ROOT"
sudo install -m 0644 /tmp/edgerun_dash_apps.wasm "$DASH_MODULES_ROOT/mail.wasm"
sudo install -m 0644 /tmp/edgerun_dash_apps.wasm "$DASH_MODULES_ROOT/git.wasm"
sudo install -m 0644 /tmp/edgerun_dash_apps.wasm "$DASH_MODULES_ROOT/blog.wasm"
cd "$REMOTE_CHECKOUT"
sudo git fetch "$REMOTE_REPO" "$BRANCH"
sudo git reset --hard FETCH_HEAD
sudo chown -R edgerun-server:edgerun-server "$REMOTE_CHECKOUT"
sudo -u edgerun-server /usr/local/bin/edgerun-blog generate \
    --root "$REMOTE_CHECKOUT" \
    --content-dir docs/blog \
    --out "$REMOTE_BLOG_OUT" \
    --title "$BLOG_TITLE" \
    --description "$BLOG_DESCRIPTION" \
    --base-url "$BLOG_BASE_URL"
sudo -u edgerun-server /usr/local/bin/edgerun-blog generate \
    --root "$REMOTE_CHECKOUT" \
    --content-dir docs/blog \
    --out "$REMOTE_BLOG_OUT" \
    --title "$BLOG_TITLE" \
    --description "$BLOG_DESCRIPTION" \
    --base-url "$BLOG_BASE_URL" \
    --check
sudo mkdir -p /etc/systemd/system/"$SERVICE".service.d
sudo tee /etc/systemd/system/"$SERVICE".service.d/web-surfaces.conf >/dev/null <<EOF
[Service]
ExecStart=
ExecStart=/usr/local/bin/edgerun-server --config /etc/edgerun/server/server.yaml --blog-host $BLOG_HOST --blog-root $REMOTE_CHECKOUT --blog-content-dir docs/blog --blog-static-root $REMOTE_BLOG_OUT --blog-title "$BLOG_TITLE" --blog-description "$BLOG_DESCRIPTION" --blog-base-url $BLOG_BASE_URL --git-host $GIT_HOST --git-root /srv/git --git-title "$GIT_TITLE" --git-base-url $GIT_BASE_URL --dash-host $DASH_HOST --dash-modules-root $DASH_MODULES_ROOT
EOF
sudo systemctl daemon-reload
sudo systemctl restart "$SERVICE"
systemctl is-active "$SERVICE"
REMOTE

printf 'released %s to %s\n' "$SHORT_COMMIT" "$HOST"
