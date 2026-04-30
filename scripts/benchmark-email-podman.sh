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
    echo "usage: $0 build|config|metrics|run|bench-smtp|bench-imap|stop|notes STACK" >&2
    echo "stacks: edgerun postfix exim opensmtpd dovecot stalwart" >&2
    exit 2
fi

case "$stack" in
    edgerun|postfix|exim|opensmtpd|dovecot|stalwart) ;;
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
podman_build_opts="${EDGERUN_EMAIL_BENCH_PODMAN_BUILD_OPTS:---isolation chroot}"
podman_run_opts="${EDGERUN_EMAIL_BENCH_PODMAN_RUN_OPTS:---cgroup-manager=cgroupfs}"
edgerun_bin="${EDGERUN_EMAIL_BENCH_EDGERUN_BIN:-}"

mkdir -p "$work"

find_edgerun_bin() {
    if [ -n "$edgerun_bin" ]; then
        printf '%s\n' "$edgerun_bin"
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

write_exim() {
    cat > "$work/Containerfile" <<'EOF'
FROM docker.io/library/debian:trixie-slim
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends exim4-daemon-light ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -s /usr/sbin/nologin bench \
 && mkdir -p /var/spool/mail \
 && printf '%s\n' \
      'primary_hostname = benchmark.local' \
      'domainlist local_domains = example.test' \
      'hostlist relay_from_hosts = 127.0.0.1 : 10.0.0.0/8 : 172.16.0.0/12 : 192.168.0.0/16' \
      'acl_smtp_rcpt = acl_check_rcpt' \
      'begin acl' \
      'acl_check_rcpt:' \
      '  accept domains = +local_domains' \
      '  deny message = external delivery disabled in benchmark' \
      'begin routers' \
      'local_user:' \
      '  driver = accept' \
      '  domains = +local_domains' \
      '  check_local_user' \
      '  transport = local_delivery' \
      'begin transports' \
      'local_delivery:' \
      '  driver = appendfile' \
      '  file = /var/spool/mail/$local_part' \
      '  delivery_date_add' \
      '  envelope_to_add' \
      '  return_path_add' \
      'begin retry' \
      'begin rewrite' \
      'begin authenticators' \
      > /etc/exim4/benchmark.conf
EXPOSE 25
CMD ["sh", "-c", "exim -bd -C /etc/exim4/benchmark.conf && tail -f /dev/null"]
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
        exim) write_exim ;;
        opensmtpd) write_opensmtpd ;;
        dovecot) write_dovecot ;;
        stalwart) write_stalwart ;;
    esac
}

stop_stack() {
    podman rm -f "$name" >/dev/null 2>&1 || true
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
        stalwart)
            mkdir -p "$work/data"
            podman run -d --name "$name" --replace \
                $podman_run_opts \
                -p "127.0.0.1:$smtp_port:25" \
                -p "127.0.0.1:$imap_port:143" \
                -p "127.0.0.1:18080:8080" \
                -v "$work/data:/opt/stalwart:Z" \
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
        exim)
            cat <<'EOF'
Exim: medium/high difficulty. It is compact as an MTA, but the minimal config
file is less obvious. Host install conflicts through mail-transport-agent.
Local-only benchmark config must explicitly accept only local domains and deny
external delivery.
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
        stalwart)
            cat <<'EOF'
Stalwart: modern integrated comparison target. Official Docker documentation
uses stalwartlabs/stalwart:latest and exposes SMTP/IMAP/admin ports. It is easy
to start as a container, but a fair benchmark still needs first-run domain,
account, storage, and anti-relay configuration before SMTP/IMAP load tests.
EOF
            ;;
    esac
}

metrics() {
    write_containerfile
    if [ "$stack" = "edgerun" ]; then
        config_lines="$(awk 'NF && $1 !~ /^#/ { n++ } END { print n + 0 }' "$work/server.yaml")"
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
        exim)
            cat <<EOF
stack=exim
shape=smtp_mta_only
components=1
container_base=debian:trixie-slim
packages=exim4-daemon-light,ca-certificates
config_lines=$config_lines
services=exim
ports=smtp:25
multitenancy_note=flexible router/transport model, but isolated tenant policy needs careful explicit config.
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
multitenancy_note=integrated domain/account model; fair benchmark needs automated tenant/domain bootstrap first.
EOF
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
    run)
        run_stack
        ;;
    bench-smtp)
        ./scripts/benchmark-email-protocol.sh smtp \
            --host 127.0.0.1 \
            --port "$smtp_port" \
            --count "$bench_count" \
            --concurrency "$bench_concurrency" \
            --from bench@example.test \
            --to bench@example.test
        ;;
    bench-imap)
        ./scripts/benchmark-email-protocol.sh imap \
            --host 127.0.0.1 \
            --port "$imap_port" \
            --count "$bench_count" \
            --concurrency "$bench_concurrency" \
            --user bench \
            --password bench
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
