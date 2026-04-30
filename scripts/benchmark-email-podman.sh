#!/bin/sh
set -eu

# Rootless Podman benchmark runner for comparing mail stacks one at a time.
# It avoids host package conflicts and binds only localhost high ports.
#
# Usage:
#   scripts/benchmark-email-podman.sh build edgerun
#   scripts/benchmark-email-podman.sh run edgerun
#   scripts/benchmark-email-podman.sh bench-smtp edgerun
#   scripts/benchmark-email-podman.sh bench-imap edgerun
#   scripts/benchmark-email-podman.sh stop edgerun

cmd="${1:-}"
stack="${2:-}"

if [ -z "$cmd" ] || [ -z "$stack" ]; then
    echo "usage: $0 build|config|metrics|versions|run|bench-smtp|bench-imap|stop|notes STACK" >&2
    echo "stacks: edgerun postfix opensmtpd dovecot maddy mox stalwart" >&2
    exit 2
fi

case "$stack" in
    edgerun|postfix|opensmtpd|dovecot|maddy|mox|stalwart) ;;
    *) echo "unknown stack: $stack" >&2; exit 2 ;;
esac

if ! command -v podman >/dev/null 2>&1; then
    echo "podman is required" >&2
    exit 1
fi

root="${EDGERUN_EMAIL_BENCH_ROOT:-$HOME/.cache/edgerun-email-bench}"
work="$root/$stack"
image="localhost/edgerun-email-bench-$stack:latest"
name="edgerun-email-bench-$stack"
smtp_port="${EDGERUN_EMAIL_BENCH_SMTP_PORT:-2525}"
imap_port="${EDGERUN_EMAIL_BENCH_IMAP_PORT:-1143}"
bench_count="${EDGERUN_EMAIL_BENCH_COUNT:-100}"
bench_concurrency="${EDGERUN_EMAIL_BENCH_CONCURRENCY:-4}"
podman_build_opts="${EDGERUN_EMAIL_BENCH_PODMAN_BUILD_OPTS:---no-cache --isolation chroot}"
podman_run_opts="${EDGERUN_EMAIL_BENCH_PODMAN_RUN_OPTS:---cgroup-manager=cgroupfs}"
edgerun_bin="${EDGERUN_EMAIL_BENCH_EDGERUN_BIN:-}"
smtp_bench_bin="${EDGERUN_EMAIL_BENCH_SMTP_BENCH_BIN:-}"
smtp_driver="${EDGERUN_EMAIL_BENCH_SMTP_DRIVER:-auto}"
stalwart_cli="${EDGERUN_EMAIL_BENCH_STALWART_CLI:-}"
stalwart_admin_secret_file="$work/stalwart-admin-secret"

mkdir -p "$work"

find_edgerun_bin() {
    if [ -n "$edgerun_bin" ]; then
        printf '%s\n' "$edgerun_bin"
    elif [ "${EDGERUN_EMAIL_BENCH_BUILD_EDGERUN_BIN:-1}" = "1" ] \
        && [ -f Cargo.toml ] \
        && command -v cargo >/dev/null 2>&1; then
        cargo build -p edgerun-server --release --bin edgerun-server \
            --features 'std smtp imap dns tls' >&2
        if [ -x target/x86_64-unknown-linux-musl/release/edgerun-server ]; then
            printf '%s\n' "target/x86_64-unknown-linux-musl/release/edgerun-server"
        elif [ -x target/release/edgerun-server ]; then
            printf '%s\n' "target/release/edgerun-server"
        else
            return 1
        fi
    elif command -v edgerun-server >/dev/null 2>&1; then
        command -v edgerun-server
    elif [ -x target/x86_64-unknown-linux-musl/release/edgerun-server ]; then
        printf '%s\n' "target/x86_64-unknown-linux-musl/release/edgerun-server"
    elif [ -x target/release/edgerun-server ]; then
        printf '%s\n' "target/release/edgerun-server"
    else
        return 1
    fi
}

find_smtp_bench_bin() {
    if [ -n "$smtp_bench_bin" ]; then
        printf '%s\n' "$smtp_bench_bin"
    elif command -v edgerun-smtp-bench >/dev/null 2>&1; then
        command -v edgerun-smtp-bench
    elif [ -x target/x86_64-unknown-linux-musl/release/edgerun-smtp-bench ]; then
        printf '%s\n' "target/x86_64-unknown-linux-musl/release/edgerun-smtp-bench"
    elif [ -x target/release/edgerun-smtp-bench ]; then
        printf '%s\n' "target/release/edgerun-smtp-bench"
    else
        return 1
    fi
}

find_stalwart_cli() {
    if [ -n "$stalwart_cli" ]; then
        printf '%s\n' "$stalwart_cli"
    elif command -v stalwart-cli >/dev/null 2>&1; then
        command -v stalwart-cli
    elif [ -x "$root/tools/stalwart-cli-v1.0.4/stalwart-cli-x86_64-unknown-linux-gnu/stalwart-cli" ]; then
        printf '%s\n' "$root/tools/stalwart-cli-v1.0.4/stalwart-cli-x86_64-unknown-linux-gnu/stalwart-cli"
    else
        return 1
    fi
}

ensure_stalwart_cli() {
    if cli="$(find_stalwart_cli 2>/dev/null)"; then
        printf '%s\n' "$cli"
        return
    fi
    tool_dir="$root/tools/stalwart-cli-v1.0.4"
    archive="stalwart-cli-x86_64-unknown-linux-gnu.tar.xz"
    mkdir -p "$tool_dir"
    (
        cd "$tool_dir"
        curl -fL -o "$archive" "https://github.com/stalwartlabs/cli/releases/download/v1.0.4/$archive"
        curl -fL -o "$archive.sha256" "https://github.com/stalwartlabs/cli/releases/download/v1.0.4/$archive.sha256"
        sha256sum -c "$archive.sha256"
        tar -xJf "$archive"
    ) >&2
    printf '%s\n' "$tool_dir/stalwart-cli-x86_64-unknown-linux-gnu/stalwart-cli"
}

run_smtp_benchmark() {
    smtp_from="bench@example.test"
    smtp_to="bench@example.test"
    if [ "$stack" = "mox" ]; then
        smtp_from="mox@localhost"
        smtp_to="mox@localhost"
    fi
    case "$smtp_driver" in
        auto)
            if bench_bin="$(find_smtp_bench_bin 2>/dev/null)"; then
                "$bench_bin" \
                    --host 127.0.0.1 \
                    --port "$smtp_port" \
                    --count "$bench_count" \
                    --concurrency "$bench_concurrency" \
                    --from "$smtp_from" \
                    --to "$smtp_to"
                return
            fi
            ;;
        rust)
            bench_bin="$(find_smtp_bench_bin)" || {
                echo "edgerun-smtp-bench binary not found; build edgerun-email --bin edgerun-smtp-bench first or set EDGERUN_EMAIL_BENCH_SMTP_BENCH_BIN" >&2
                exit 1
            }
            "$bench_bin" \
                --host 127.0.0.1 \
                --port "$smtp_port" \
                --count "$bench_count" \
                --concurrency "$bench_concurrency" \
                --from "$smtp_from" \
                --to "$smtp_to"
            return
            ;;
        shell) ;;
        *)
            echo "unknown EDGERUN_EMAIL_BENCH_SMTP_DRIVER: $smtp_driver" >&2
            exit 2
            ;;
    esac

    ./scripts/benchmark-email-protocol.sh smtp \
        --host 127.0.0.1 \
        --port "$smtp_port" \
        --count "$bench_count" \
        --concurrency "$bench_concurrency" \
        --from "$smtp_from" \
        --to "$smtp_to"
}

run_imap_benchmark() {
    user="bench"
    password="bench"
    if [ "$stack" = "mox" ]; then
        user="mox@localhost"
        password="moxmoxmox"
    elif [ "$stack" = "stalwart" ]; then
        user="bench@example.test"
        password="bench-Email-2026"
    fi
    ./scripts/benchmark-email-protocol.sh imap \
        --host 127.0.0.1 \
        --port "$imap_port" \
        --count "$bench_count" \
        --concurrency "$bench_concurrency" \
        --user "$user" \
        --password "$password"
}

write_edgerun() {
    bin="$(find_edgerun_bin)" || {
        echo "edgerun-server binary not found; build edgerun-server first or set EDGERUN_EMAIL_BENCH_EDGERUN_BIN" >&2
        exit 1
    }
    cp "$bin" "$work/edgerun-server"
    cat > "$work/server.yaml" <<'EOF'
apiVersion: edgerun.io/v1alpha1
kind: SmtpServer
metadata:
  name: benchmark
spec:
  hostname: benchmark.local
  bind_address: "0.0.0.0:25"
  smtps: false
  starttls: false
  local_domains:
    - example.test
  maildir_root: /data/maildirs
  relay_enabled: false
  users:
    - username: bench
      domains:
        - example.test
---
apiVersion: edgerun.io/v1alpha1
kind: ImapServer
metadata:
  name: benchmark
spec:
  hostname: benchmark.local
  bind_address: "0.0.0.0:143"
  imaps: false
  maildir_root: /data/maildirs
  users:
    - username: bench
      password: bench
EOF
    cat > "$work/Containerfile" <<'EOF'
FROM scratch
COPY edgerun-server /edgerun-server
COPY server.yaml /server.yaml
EXPOSE 25 143
ENTRYPOINT ["/edgerun-server", "--config", "/server.yaml"]
EOF
}

write_postfix() {
    cat > "$work/Containerfile" <<'EOF'
FROM docker.io/library/debian:trixie-slim
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends postfix ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -s /usr/sbin/nologin bench \
 && postconf -e 'myhostname = benchmark.local' \
 && postconf -e 'myorigin = example.test' \
 && postconf -e 'mydestination = example.test, benchmark.local, localhost.localdomain, localhost' \
 && postconf -e 'inet_interfaces = all' \
 && postconf -e 'inet_protocols = ipv4' \
 && postconf -e 'mynetworks = 127.0.0.0/8 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16' \
 && postconf -e 'relay_domains =' \
 && postconf -e 'default_transport = error:no external delivery in benchmark' \
 && postconf -e 'relay_transport = error:no relay in benchmark' \
 && postconf -e 'smtpd_recipient_restrictions = permit_mynetworks,reject_unauth_destination' \
 && postconf -e 'maillog_file = /dev/stdout'
EXPOSE 25
CMD ["postfix", "start-fg"]
EOF
}

write_opensmtpd() {
    cat > "$work/Containerfile" <<'EOF'
FROM docker.io/library/debian:trixie-slim
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends opensmtpd ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -s /usr/sbin/nologin bench \
 && mkdir -p /home/bench/Maildir \
 && chown -R bench:bench /home/bench/Maildir \
 && printf '%s\n' \
      'listen on 0.0.0.0 port 25' \
      'action "local_maildir" maildir "/home/%{user.username}/Maildir"' \
      'match from any for domain "example.test" action "local_maildir"' \
      'match from any reject' \
      > /etc/smtpd.conf
EXPOSE 25
CMD ["smtpd", "-d", "-f", "/etc/smtpd.conf"]
EOF
}

write_dovecot() {
    cat > "$work/Containerfile" <<'EOF'
FROM docker.io/library/debian:trixie-slim
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends dovecot-core dovecot-imapd ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -s /usr/sbin/nologin bench \
 && mkdir -p /home/bench/Maildir/{cur,new,tmp} \
 && chown -R bench:bench /home/bench/Maildir \
 && printf 'bench:{PLAIN}bench:1000:1000::/home/bench::\n' > /etc/dovecot/users \
 && printf '%s\n' \
      'dovecot_config_version = 2.4.0' \
      'dovecot_storage_version = 2.4.0' \
      'protocols = imap' \
      'listen = *' \
      'mail_driver = maildir' \
      'mail_path = ~/Maildir' \
      'auth_allow_cleartext = yes' \
      'auth_mechanisms = plain login' \
      'passdb passwd-file {' \
      '  default_password_scheme = plain' \
      '  auth_username_format = %{user}' \
      '  passwd_file_path = /etc/dovecot/users' \
      '}' \
      'userdb passwd-file {' \
      '  auth_username_format = %{user}' \
      '  passwd_file_path = /etc/dovecot/users' \
      '}' \
      'service imap-login {' \
      '  inet_listener imap {' \
      '    port = 143' \
      '  }' \
      '}' \
      > /etc/dovecot/dovecot.conf
EXPOSE 143
CMD ["dovecot", "-F"]
EOF
}

write_maddy() {
    cat > "$work/Containerfile" <<'EOF'
FROM docker.io/foxcpp/maddy:latest
RUN cat > /data/maddy.conf <<'MADDYCONF'
$(primary_domain) = example.test
$(local_domains) = $(primary_domain)

hostname benchmark.local
tls off

auth.pass_table local_authdb {
    table sql_table {
        driver sqlite3
        dsn credentials.db
        table_name passwords
    }
}

storage.imapsql local_mailboxes {
    driver sqlite3
    dsn imapsql.db
}

table.chain local_rewrites {
    optional_step regexp "(.+)\\+(.+)@(.+)" "$1@$3"
    optional_step static {
        entry postmaster postmaster@$(primary_domain)
    }
}

msgpipeline local_routing {
    destination postmaster $(local_domains) {
        modify {
            replace_rcpt &local_rewrites
        }
        deliver_to &local_mailboxes
    }
    default_destination {
        reject 550 5.1.1 "User doesn't exist"
    }
}

smtp tcp://0.0.0.0:25 {
    limits {
        all concurrency 256
        all rate 10000 1s
    }
    default_source {
        destination postmaster $(local_domains) {
            deliver_to &local_routing
        }
        default_destination {
            reject 550 5.1.1 "User doesn't exist"
        }
    }
}

imap tcp://0.0.0.0:143 {
    auth &local_authdb
    storage &local_mailboxes
}
MADDYCONF
RUN maddy --config /data/maddy.conf verify-config \
 && maddy --config /data/maddy.conf creds create -p bench bench@example.test \
 && maddy --config /data/maddy.conf imap-acct create bench@example.test \
 && maddy --config /data/maddy.conf creds create -p bench bench \
 && maddy --config /data/maddy.conf imap-acct create bench
EXPOSE 25 143
CMD ["--config", "/data/maddy.conf", "run"]
EOF
}

write_mox() {
    cat > "$work/Containerfile" <<'EOF'
FROM r.xmox.nl/mox:latest
RUN mkdir -p /data
RUN printf '%s\n' \
  '#!/bin/sh' \
  'set -eu' \
  'dir=/data/localserve' \
  'if [ ! -f "$dir/mox.conf" ]; then' \
  '  mox localserve -initonly -dir "$dir" -ip 0.0.0.0' \
  '  awk '\''BEGIN { skip = 0 } /^\t\tJunkFilter:/ { skip = 1; next } skip && /^\t\tNoFirstTimeSenderDelay:/ { skip = 0 } !skip { print }'\'' "$dir/domains.conf" > "$dir/domains.conf.tmp"' \
  '  mv "$dir/domains.conf.tmp" "$dir/domains.conf"' \
  'fi' \
  'exec mox localserve -dir "$dir"' \
  > /usr/local/bin/run-mox-localserve \
 && chmod +x /usr/local/bin/run-mox-localserve
EXPOSE 1025 1143 1080 1443
CMD ["/usr/local/bin/run-mox-localserve"]
EOF
}

write_stalwart() {
    cat > "$work/Containerfile" <<'EOF'
FROM docker.io/stalwartlabs/stalwart:latest
EOF
}

write_containerfile() {
    case "$stack" in
        edgerun) write_edgerun ;;
        postfix) write_postfix ;;
        opensmtpd) write_opensmtpd ;;
        dovecot) write_dovecot ;;
        maddy) write_maddy ;;
        mox) write_mox ;;
        stalwart) write_stalwart ;;
    esac
}

stop_stack() {
    podman rm -f "$name" >/dev/null 2>&1 || true
}

prepare_stalwart_volumes() {
    mkdir -p "$work/etc" "$work/data"
    podman unshare chown -R 2000:2000 "$work/etc" "$work/data" 2>/dev/null \
        || chmod -R u+rwX,go+rwX "$work/etc" "$work/data" 2>/dev/null \
        || true
}

wait_stalwart_http() {
    user="$1"
    password="$2"
    deadline="$(( $(date +%s) + 30 ))"
    while [ "$(date +%s)" -lt "$deadline" ]; do
        if curl -fsS -u "$user:$password" "http://127.0.0.1:18080/api/schema" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.5
    done
    return 1
}

bootstrap_stalwart() {
    cli="$(ensure_stalwart_cli)"
    if [ -e "$work/etc" ] || [ -e "$work/data" ]; then
        podman unshare rm -rf "$work/etc" "$work/data" 2>/dev/null || {
            chown -R "$(id -u):$(id -g)" "$work/etc" "$work/data" 2>/dev/null || true
            rm -rf "$work/etc" "$work/data"
        }
    fi
    prepare_stalwart_volumes
    stop_stack
    podman run -d --name "$name" --replace \
        $podman_run_opts \
        -e STALWART_RECOVERY_ADMIN=admin:admin \
        -p "127.0.0.1:18080:8080" \
        -v "$work/etc:/etc/stalwart:Z" \
        -v "$work/data:/var/lib/stalwart:Z" \
        "$image" >/dev/null
    wait_stalwart_http admin admin || {
        podman logs "$name" 2>&1 || true
        return 1
    }

    bootstrap_patch="$work/bootstrap.json"
    cat > "$bootstrap_patch" <<'EOF'
{
  "serverHostname": "benchmark.local",
  "defaultDomain": "example.test",
  "requestTlsCertificate": false,
  "generateDkimKeys": false,
  "dataStore": {
    "@type": "RocksDb",
    "path": "/var/lib/stalwart/data",
    "blobSize": 16834,
    "bufferSize": 134217728,
    "poolWorkers": null
  },
  "blobStore": {"@type": "Default"},
  "searchStore": {"@type": "Default"},
  "inMemoryStore": {"@type": "Default"},
  "directory": {"@type": "Internal"},
  "tracer": {
    "@type": "Stdout",
    "ansi": false,
    "buffered": false,
    "enable": true,
    "events": {},
    "eventsPolicy": "exclude",
    "level": "warn",
    "lossy": false,
    "multiline": false
  },
  "dnsServer": {"@type": "Manual"}
}
EOF
    setup_out="$work/bootstrap.out"
    "$cli" --url http://127.0.0.1:18080 --user admin --password admin \
        update Bootstrap singleton --file "$bootstrap_patch" > "$setup_out"
    admin_secret="$(sed -n 's/^  secret: "\(.*\)"$/\1/p' "$setup_out" | head -1)"
    if [ -z "$admin_secret" ]; then
        cat "$setup_out" >&2
        return 1
    fi
    printf '%s\n' "$admin_secret" > "$stalwart_admin_secret_file"

    stop_stack
    podman run -d --name "$name" --replace \
        $podman_run_opts \
        -p "127.0.0.1:18080:8080" \
        -v "$work/etc:/etc/stalwart:Z" \
        -v "$work/data:/var/lib/stalwart:Z" \
        "$image" >/dev/null
    wait_stalwart_http admin@example.test "$admin_secret" || {
        podman logs "$name" 2>&1 || true
        return 1
    }

    account_json="$work/account-bench.json"
    cat > "$account_json" <<'EOF'
{
  "name": "bench",
  "domainId": "b",
  "description": "Benchmark User",
  "credentials": {
    "0": {"@type": "Password", "secret": "bench-Email-2026", "expiresAt": null, "allowedIps": {}}
  },
  "aliases": {},
  "memberGroupIds": {},
  "permissions": {"@type": "Inherit"},
  "roles": {"@type": "User"},
  "quotas": {},
  "locale": "en_US",
  "encryptionAtRest": {"@type": "Disabled"}
}
EOF
    "$cli" --url http://127.0.0.1:18080 --user admin@example.test --password "$admin_secret" \
        create Account/User --file "$account_json" >/dev/null

    imap_json="$work/listener-imap.json"
    cat > "$imap_json" <<'EOF'
{
  "name": "imap",
  "bind": {"[::]:143": true},
  "protocol": "imap",
  "tlsImplicit": false,
  "useTls": false,
  "socketReuseAddress": true,
  "socketReusePort": false,
  "socketNoDelay": true,
  "overrideProxyTrustedNetworks": {},
  "tlsDisableCipherSuites": {},
  "tlsDisableProtocols": {},
  "tlsIgnoreClientOrder": false
}
EOF
    "$cli" --url http://127.0.0.1:18080 --user admin@example.test --password "$admin_secret" \
        create NetworkListener --file "$imap_json" >/dev/null
    "$cli" --url http://127.0.0.1:18080 --user admin@example.test --password "$admin_secret" \
        update Imap singleton --json '{"allowPlainTextAuth":true}' >/dev/null
    stop_stack
    prepare_stalwart_volumes
}

run_stack() {
    stop_stack
    case "$stack" in
        edgerun)
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$smtp_port:25" \
                -p "127.0.0.1:$imap_port:143" "$image" >/dev/null
            ;;
        dovecot)
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$imap_port:143" "$image" >/dev/null
            ;;
        maddy)
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$smtp_port:25" \
                -p "127.0.0.1:$imap_port:143" "$image" >/dev/null
            ;;
        mox)
            mkdir -p "$work/data"
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$smtp_port:1025" \
                -p "127.0.0.1:$imap_port:1143" \
                -p "127.0.0.1:18080:1080" \
                -v "$work/data:/data:Z" \
                "$image" >/dev/null
            ;;
        stalwart)
            if [ ! -f "$work/etc/config.json" ]; then
                bootstrap_stalwart
            fi
            prepare_stalwart_volumes
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$smtp_port:25" \
                -p "127.0.0.1:$imap_port:143" \
                -p "127.0.0.1:18080:8080" \
                -v "$work/etc:/etc/stalwart:Z" \
                -v "$work/data:/var/lib/stalwart:Z" \
                "$image" >/dev/null
            ;;
        *)
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$smtp_port:25" "$image" >/dev/null
            ;;
    esac
    sleep 2
    podman ps --filter "name=$name"
    if ! podman ps --filter "name=$name" --format '{{.Names}}' | grep -qx "$name"; then
        podman logs "$name" 2>&1 || true
        return 1
    fi
}

notes() {
    case "$stack" in
        edgerun)
            cat <<'EOF'
Edgerun: integrated reference server. The rootless benchmark image copies one
static edgerun-server binary plus one server.yaml into a scratch image. SMTP and
IMAP run in the same process, with no external package manager or sidecar
services in this benchmark shape.
EOF
            ;;
        postfix)
            cat <<'EOF'
Postfix: medium difficulty. Debian package is easy, but it conflicts with other
mail-transport-agent packages on a host. Local-only benchmark config needs
main.cf changes to disable default/relay transport and reject relay attempts.
IMAP requires Dovecot or another separate service for a comparable stack.
EOF
            ;;
        opensmtpd)
            cat <<'EOF'
OpenSMTPD: low/medium difficulty for SMTP-only. The local-only config is small
and readable. Host install conflicts through mail-transport-agent. IMAP still
requires a separate service.
EOF
            ;;
        dovecot)
            cat <<'EOF'
Dovecot: medium difficulty as the IMAP half of traditional stacks. It does not
conflict with MTAs, but a benchmark needs explicit passdb/userdb and Maildir
configuration to avoid relying on host PAM/users.
EOF
            ;;
        maddy)
            cat <<'EOF'
Maddy: integrated SMTP/IMAP server. The benchmark image uses the official
foxcpp/maddy image with a local-only no-TLS test config, one local domain, and
one benchmark account created during image build.
EOF
            ;;
        mox)
            cat <<'EOF'
Mox: modern integrated mail server. The benchmark image uses Mox localserve,
which is intended for local email software development and testing. It accepts
local SMTP on port 1025 and exposes IMAP on port 1143 with the built-in
mox@localhost account. Treat these measurements as localserve evidence until
production quickstart/domain/account setup is automated reproducibly.
EOF
            ;;
        stalwart)
            cat <<'EOF'
Stalwart: modern integrated comparison target. Official Docker documentation
uses stalwartlabs/stalwart:latest and exposes SMTP/IMAP/admin ports. The v0.16
container starts in bootstrap mode when no generated setup config exists. The
benchmark harness bootstraps a local example.test domain, one account, plain
IMAP on port 143, and clear-text IMAP auth through stalwart-cli. Current local
SMTP smoke is reliable only at low concurrency; higher concurrency needs more
tuning before it is publishable.
EOF
            ;;
    esac
}

metrics() {
    write_containerfile
    if [ "$stack" = "edgerun" ]; then
        config_lines="$(awk 'NF && $1 !~ /^#/ { n++ } END { print n + 0 }' "$work/server.yaml")"
    elif [ "$stack" = "maddy" ]; then
        config_lines="$(awk '
            /<<'\''MADDYCONF'\''/ { in_conf = 1; next }
            /^MADDYCONF$/ { in_conf = 0; next }
            in_conf && NF && $1 !~ /^#/ { n++ }
            END { print n + 0 }
        ' "$work/Containerfile")"
    else
        config_lines="$(awk '
            /postconf -e/ { n++ }
            /^[[:space:]]*'\''[^'\'']+'\''[[:space:]]*\\/ { n++ }
            END { print n + 0 }
        ' "$work/Containerfile")"
    fi
    case "$stack" in
        edgerun)
            binary_bytes=0
            if bin="$(find_edgerun_bin 2>/dev/null)"; then
                binary_bytes="$(stat -c '%s' "$bin" 2>/dev/null || printf '0')"
            fi
            cat <<EOF
stack=edgerun
shape=integrated_reference_server
components=1
container_base=scratch
packages=none
binary_bytes=$binary_bytes
config_lines=$config_lines
services=edgerun-server
ports=smtp:25,imap:143
multitenancy_note=single process with explicit local domains and per-user mailbox roots; tenant policy should become a first-class config dimension before broad hosting claims.
EOF
            ;;
        postfix)
            cat <<EOF
stack=postfix
shape=smtp_mta_only
components=1
container_base=debian:trixie-slim
packages=postfix,ca-certificates
config_lines=$config_lines
services=postfix
ports=smtp:25
multitenancy_note=domain and mailbox policy live in MTA maps/config; IMAP/webmail require separate components.
EOF
            ;;
        opensmtpd)
            cat <<EOF
stack=opensmtpd
shape=smtp_mta_only
components=1
container_base=debian:trixie-slim
packages=opensmtpd,ca-certificates
config_lines=$config_lines
services=smtpd
ports=smtp:25
multitenancy_note=readable rule model; IMAP, webmail, and tenant mailbox access remain separate.
EOF
            ;;
        dovecot)
            cat <<EOF
stack=dovecot
shape=imap_only
components=1
container_base=debian:trixie-slim
packages=dovecot-core,dovecot-imapd,ca-certificates
config_lines=$config_lines
services=dovecot
ports=imap:143
multitenancy_note=strong mailbox component; tenant isolation depends on auth/userdb/mail location design.
EOF
            ;;
        maddy)
            cat <<EOF
stack=maddy
shape=integrated_mail_server
components=1
container_base=foxcpp/maddy:latest
packages=image-provided
config_lines=$config_lines
services=maddy
ports=smtp:25,imap:143
multitenancy_note=single binary with explicit local domain and account DB; benchmark config disables TLS only for local test isolation.
EOF
            ;;
        mox)
            cat <<EOF
stack=mox
shape=integrated_localserve_mail_server
components=1
container_base=r.xmox.nl/mox:latest
packages=image-provided
config_lines=$config_lines
services=mox
ports=smtp:1025,imap:1143,http:1080,https:1443
multitenancy_note=localserve accepts all messages into a development account; fair production benchmark still needs automated quickstart/domain/account bootstrap.
EOF
            ;;
        stalwart)
            cat <<EOF
stack=stalwart
shape=integrated_mail_server
components=1
container_base=stalwartlabs/stalwart:latest
packages=image-provided
config_lines=$config_lines
services=stalwart
ports=smtp:25,imap:143,admin:8080
multitenancy_note=integrated domain/account model; benchmark automation creates one local domain and account through stalwart-cli; SMTP currently needs low-concurrency handling for stable local evidence.
EOF
            ;;
    esac
}

versions() {
    case "$stack" in
        edgerun)
            printf 'stack=edgerun\n'
            source_version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml 2>/dev/null | head -1)"
            printf 'source_version=%s\n' "${source_version:-unknown}"
            podman image inspect "$image" --format 'image_id={{.Id}} image_created={{.Created}} image_size={{.Size}}' 2>/dev/null || true
            ;;
        postfix)
            podman run --rm "$image" sh -lc 'postconf mail_version; dpkg-query -W postfix ca-certificates'
            ;;
        opensmtpd)
            podman run --rm "$image" sh -lc 'smtpd -h 2>&1 | sed -n "1p"; dpkg-query -W opensmtpd ca-certificates'
            ;;
        dovecot)
            podman run --rm "$image" sh -lc 'printf "dovecot_version=%s\n" "$(dovecot --version)"; dpkg-query -W dovecot-core dovecot-imapd ca-certificates'
            ;;
        maddy)
            podman run --rm --entrypoint sh "$image" -lc 'maddy version | sed -n "1p"'
            ;;
        mox)
            podman run --rm --entrypoint sh "$image" -lc 'mox version'
            ;;
        stalwart)
            podman run --rm --entrypoint sh "$image" -lc 'stalwart --version 2>/dev/null | sed -n "1p" || /usr/local/bin/stalwart --version 2>/dev/null | sed -n "1p" || true'
            ;;
    esac
}

case "$cmd" in
    build)
        write_containerfile
        podman build $podman_build_opts -t "$image" -f "$work/Containerfile" "$work"
        ;;
    config)
        write_containerfile
        cat "$work/Containerfile"
        if [ "$stack" = "edgerun" ]; then
            printf '\n# --- server.yaml ---\n'
            cat "$work/server.yaml"
        fi
        ;;
    metrics)
        metrics
        ;;
    versions)
        versions
        ;;
    run)
        run_stack
        ;;
    bench-smtp)
        run_smtp_benchmark
        ;;
    bench-imap)
        run_imap_benchmark
        ;;
    stop)
        stop_stack
        ;;
    notes)
        notes
        ;;
    *)
        echo "unknown command: $cmd" >&2
        exit 2
        ;;
esac
