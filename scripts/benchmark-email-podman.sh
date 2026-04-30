#!/bin/sh
set -eu

# Rootless Podman benchmark runner for comparing mail stacks one at a time.
# It avoids host package conflicts and binds only localhost high ports.
#
# Usage:
#   scripts/benchmark-email-podman.sh build postfix
#   scripts/benchmark-email-podman.sh run postfix
#   scripts/benchmark-email-podman.sh bench-smtp postfix
#   scripts/benchmark-email-podman.sh stop postfix

cmd="${1:-}"
stack="${2:-}"

if [ -z "$cmd" ] || [ -z "$stack" ]; then
    echo "usage: $0 build|run|bench-smtp|bench-imap|stop|notes STACK" >&2
    echo "stacks: postfix exim opensmtpd dovecot stalwart" >&2
    exit 2
fi

case "$stack" in
    postfix|exim|opensmtpd|dovecot|stalwart) ;;
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

mkdir -p "$work"

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
 && printf '%s\n' \
      'protocols = imap' \
      'listen = *' \
      'mail_driver = maildir' \
      'mail_path = ~/Maildir' \
      'auth_allow_cleartext = yes' \
      'auth_mechanisms = plain login' \
      'passdb static {' \
      '  driver = static' \
      '  fields {' \
      '    password = bench' \
      '  }' \
      '}' \
      'userdb static {' \
      '  driver = static' \
      '  fields {' \
      '    uid = bench' \
      '    gid = bench' \
      '    home = /home/bench' \
      '  }' \
      '}' \
      'service imap-login {' \
      '  inet_listener imap {' \
      '    port = 143' \
      '  }' \
      '}' \
      > /etc/dovecot/local.conf
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

case "$cmd" in
    build)
        write_containerfile
        podman build $podman_build_opts -t "$image" -f "$work/Containerfile" "$work"
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
