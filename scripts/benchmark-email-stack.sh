#!/bin/sh
set -eu

# Print a reproducible, no-dependency baseline for Edgerun and common mail
# stacks on the same host. Pass a host name to run through ssh:
#   scripts/benchmark-email-stack.sh mail

if [ "${1:-}" != "--local" ] && [ "${1:-}" != "" ]; then
    exec ssh "$1" 'sh -s -- --local' < "$0"
fi

now_utc() {
    date -u '+%Y-%m-%dT%H:%M:%SZ'
}

kv() {
    printf '%s=%s\n' "$1" "$2"
}

read_os() {
    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        kv host_os "${NAME:-unknown} ${VERSION_ID:-unknown}"
    else
        kv host_os unknown
    fi
    kv kernel "$(uname -srmo)"
    kv captured_at "$(now_utc)"
    kv uptime "$(uptime | sed 's/^ *//')"
}

show_smaps_rollup() {
    pid="$1"
    if [ "$pid" = "0" ] || [ ! -r "/proc/$pid/smaps_rollup" ]; then
        return
    fi
    awk '
        /^(Rss|Pss|Private_Clean|Private_Dirty|Shared_Clean|Shared_Dirty|Swap):/ {
            key=$1
            sub(":", "", key)
            printf "proc_%s_kb=%s\n", tolower(key), $2
        }
    ' "/proc/$pid/smaps_rollup" 2>/dev/null || kv smaps_rollup unreadable
}

show_unit() {
    label="$1"
    unit="$2"
    printf '\n## %s\n' "$label"
    kv unit "$unit"
    if ! command -v systemctl >/dev/null 2>&1; then
        kv systemd unavailable
        return
    fi
    active="$(systemctl is-active "$unit" 2>/dev/null || true)"
    kv active "${active:-unknown}"
    systemctl show "$unit" \
        -p MainPID \
        -p MemoryCurrent \
        -p MemoryPeak \
        -p TasksCurrent \
        -p ActiveState \
        -p SubState \
        --no-pager 2>/dev/null || true
    pid="$(systemctl show "$unit" -p MainPID --value 2>/dev/null || printf '0')"
    case "$pid" in
        ''|*[!0-9]*) pid=0 ;;
    esac
    if [ "$pid" != "0" ]; then
        ps -o pid=,ppid=,comm=,rss=,vsz=,etime=,args= -p "$pid" 2>/dev/null || true
        show_smaps_rollup "$pid"
    fi
}

show_binary() {
    label="$1"
    name="$2"
    printf '\n## binary: %s\n' "$label"
    path="$(command -v "$name" 2>/dev/null || true)"
    if [ -z "$path" ]; then
        kv "$name" not_installed
        return
    fi
    kv path "$path"
    file "$path" 2>/dev/null || true
    stat -c 'binary_bytes=%s' "$path" 2>/dev/null || true
}

printf '# Email Stack Benchmark Snapshot\n\n'
read_os

printf '\n# Service memory\n'
show_unit "Edgerun all-in-one server" "edgerun-server.service"
show_unit "Postfix MTA" "postfix.service"
show_unit "Dovecot IMAP/LMTP" "dovecot.service"
show_unit "Exim MTA" "exim4.service"
show_unit "OpenSMTPD MTA" "opensmtpd.service"
show_unit "OpenSMTPD alternate unit" "smtpd.service"
show_unit "Stalwart Mail Server" "stalwart-mail.service"
show_unit "Stalwart alternate unit" "stalwart.service"

printf '\n# Installed binary sizes\n'
show_binary "edgerun-server" "edgerun-server"
show_binary "edgerun-mailq" "edgerun-mailq"
show_binary "edgerun-smtp-probe" "edgerun-smtp-probe"
show_binary "postfix" "postfix"
show_binary "postfix master" "master"
show_binary "dovecot" "dovecot"
show_binary "exim" "exim"
show_binary "exim4" "exim4"
show_binary "opensmtpd" "smtpd"
show_binary "stalwart-mail" "stalwart-mail"
show_binary "stalwart" "stalwart"

printf '\n# Listening mail/http processes\n'
if command -v ss >/dev/null 2>&1; then
    ss -ltnp | awk 'NR == 1 || /edgerun-server|postfix|master|dovecot|exim|smtpd|stalwart/'
else
    kv ss unavailable
fi
