#!/usr/bin/env bash
set -eu

# No-spam SMTP/IMAP protocol benchmark. It sends only loopback/local-recipient
# traffic to a configured host:port and records per-operation latency.
#
# SMTP example:
#   scripts/benchmark-email-protocol.sh smtp --host 127.0.0.1 --port 2525 \
#     --count 100 --concurrency 4 --from bench@example.test --to bench@example.test
#
# IMAP example:
#   scripts/benchmark-email-protocol.sh imap --host 127.0.0.1 --port 1143 \
#     --count 100 --user bench --password bench

mode="${1:-}"
if [ -z "$mode" ]; then
    echo "usage: $0 smtp|imap [options]" >&2
    exit 2
fi
shift

host="127.0.0.1"
port=""
count=100
concurrency=1
from="bench@example.test"
to="bench@example.test"
user="bench"
password="bench"
message_bytes=1024
timeout=10

while [ "$#" -gt 0 ]; do
    case "$1" in
        --host) host="$2"; shift 2 ;;
        --port) port="$2"; shift 2 ;;
        --count) count="$2"; shift 2 ;;
        --concurrency) concurrency="$2"; shift 2 ;;
        --from) from="$2"; shift 2 ;;
        --to) to="$2"; shift 2 ;;
        --user) user="$2"; shift 2 ;;
        --password) password="$2"; shift 2 ;;
        --message-bytes) message_bytes="$2"; shift 2 ;;
        --timeout) timeout="$2"; shift 2 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$port" ]; then
    case "$mode" in
        smtp) port=25 ;;
        imap) port=143 ;;
        *) echo "unknown mode: $mode" >&2; exit 2 ;;
    esac
fi

if [ -z "${BASH_VERSION:-}" ]; then
    echo "bash is required for the protocol benchmark" >&2
    exit 1
fi

case "$count" in ''|*[!0-9]*) echo "count must be numeric" >&2; exit 2 ;; esac
case "$concurrency" in ''|*[!0-9]*) echo "concurrency must be numeric" >&2; exit 2 ;; esac
case "$message_bytes" in ''|*[!0-9]*) echo "message-bytes must be numeric" >&2; exit 2 ;; esac

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM
latencies="$tmpdir/latencies.tsv"
: > "$latencies"

now_ns() {
    date +%s%N
}

payload_file="$tmpdir/payload.txt"
awk -v n="$message_bytes" 'BEGIN {
    line = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    written = 0;
    while (written < n) {
        remain = n - written;
        out = substr(line, 1, remain < length(line) ? remain : length(line));
        print out;
        written += length(out) + 2;
    }
}' > "$payload_file"

smtp_once() {
    i="$1"
    rsp="$tmpdir/smtp-$i.rsp"
    mid="bench-$i-$(now_ns)@$host"
    start="$(now_ns)"
    if smtp_dialog "$i" "$mid" > "$rsp" 2>"$tmpdir/smtp-$i.err"; then
        end="$(now_ns)"
            printf '%s\t%s\n' "$i" "$(((end - start) / 1000000))" > "$tmpdir/result-$i.tsv"
        return 0
    fi
    end="$(now_ns)"
    printf '%s\tFAIL\t%s\n' "$i" "$(((end - start) / 1000000))" > "$tmpdir/result-$i.tsv"
    return 1
}

smtp_read_reply() {
    expected="$1"
    line=""
    while IFS= read -r -t "$timeout" line <&3; do
        line="${line%$'\r'}"
        printf '%s\n' "$line"
        case "$line" in
            "$expected "*) return 0 ;;
            "$expected-"*) ;;
            [0-9][0-9][0-9]' '*) return 1 ;;
        esac
    done
    return 1
}

smtp_send_expect() {
    expected="$1"
    command="$2"
    printf '%s\r\n' "$command" >&3
    smtp_read_reply "$expected"
}

smtp_dialog() {
    i="$1"
    mid="$2"
    exec 3<>"/dev/tcp/$host/$port"
    smtp_read_reply 220
    smtp_send_expect 250 "EHLO benchmark.local"
    smtp_send_expect 250 "MAIL FROM:<$from>"
    smtp_send_expect 250 "RCPT TO:<$to>"
    smtp_send_expect 354 "DATA"
    printf 'From: <%s>\r\n' "$from" >&3
    printf 'To: <%s>\r\n' "$to" >&3
    printf 'Subject: benchmark %s\r\n' "$i" >&3
    printf 'Message-ID: <%s>\r\n' "$mid" >&3
    printf 'Date: Thu, 30 Apr 2026 00:00:00 +0000\r\n' >&3
    printf '\r\n' >&3
    while IFS= read -r line; do
        line="${line%$'\r'}"
        line="${line/#./..}"
        printf '%s\r\n' "$line" >&3
    done < "$payload_file"
    printf '.\r\n' >&3
    smtp_read_reply 250
    printf 'QUIT\r\n' >&3
    smtp_read_reply 221 >/dev/null || true
    exec 3<&-
    exec 3>&-
}

imap_once() {
    i="$1"
    rsp="$tmpdir/imap-$i.rsp"
    start="$(now_ns)"
    if imap_dialog > "$rsp" 2>"$tmpdir/imap-$i.err"; then
        end="$(now_ns)"
        printf '%s\t%s\n' "$i" "$(((end - start) / 1000000))" > "$tmpdir/result-$i.tsv"
        return 0
    fi
    end="$(now_ns)"
    printf '%s\tFAIL\t%s\n' "$i" "$(((end - start) / 1000000))" > "$tmpdir/result-$i.tsv"
    return 1
}

imap_read_until_tag() {
    tag="$1"
    line=""
    while IFS= read -r -t "$timeout" line <&3; do
        line="${line%$'\r'}"
        printf '%s\n' "$line"
        case "$line" in
            "$tag OK"*) return 0 ;;
            "$tag NO"*|"$tag BAD"*) return 1 ;;
        esac
    done
    return 1
}

imap_dialog() {
    exec 3<>"/dev/tcp/$host/$port"
    IFS= read -r -t "$timeout" line <&3 || return 1
    printf '%s\n' "${line%$'\r'}"
    printf 'a001 LOGIN "%s" "%s"\r\n' "$user" "$password" >&3
    imap_read_until_tag a001
    printf 'a002 LIST "" "*"\r\n' >&3
    imap_read_until_tag a002
    printf 'a003 SELECT INBOX\r\n' >&3
    imap_read_until_tag a003
    printf 'a004 LOGOUT\r\n' >&3
    imap_read_until_tag a004 >/dev/null || true
    exec 3<&-
    exec 3>&-
}

run_one() {
    case "$mode" in
        smtp) smtp_once "$1" ;;
        imap) imap_once "$1" ;;
        *) echo "unknown mode: $mode" >&2; exit 2 ;;
    esac
}

started="$(now_ns)"
i=1
active=0
failures=0
while [ "$i" -le "$count" ]; do
    (
        set +e
        run_one "$i"
        status="$?"
        if [ ! -s "$tmpdir/result-$i.tsv" ]; then
            printf '%s\tFAIL\t0\n' "$i" > "$tmpdir/result-$i.tsv"
        fi
        exit "$status"
    ) || exit 1 &
    active=$((active + 1))
    if [ "$active" -ge "$concurrency" ]; then
        if ! wait; then
            failures=$((failures + 1))
        fi
        active=0
    fi
    i=$((i + 1))
done
if ! wait; then
    failures=$((failures + 1))
fi
ended="$(now_ns)"
find "$tmpdir" -maxdepth 1 -name 'result-*.tsv' -type f -print | sort -V | while IFS= read -r file; do
    cat "$file"
done > "$latencies"

awk -v mode="$mode" \
    -v host="$host" \
    -v port="$port" \
    -v count="$count" \
    -v concurrency="$concurrency" \
    -v elapsed_ms="$(((ended - started) / 1000000))" \
    -v batch_failures="$failures" '
    BEGIN {
        ok = 0;
        fail = 0;
    }
    $2 == "FAIL" {
        fail++;
        next;
    }
    {
        ok++;
        values[ok] = $2 + 0;
        sum += values[ok];
    }
    END {
        for (i = 1; i <= ok; i++) {
            for (j = i + 1; j <= ok; j++) {
                if (values[j] < values[i]) {
                    tmp = values[i]; values[i] = values[j]; values[j] = tmp;
                }
            }
        }
        total_fail = fail + batch_failures;
        throughput = elapsed_ms > 0 ? ok * 1000.0 / elapsed_ms : 0;
        p50 = ok ? values[int((ok - 1) * 0.50) + 1] : 0;
        p95 = ok ? values[int((ok - 1) * 0.95) + 1] : 0;
        p99 = ok ? values[int((ok - 1) * 0.99) + 1] : 0;
        avg = ok ? sum / ok : 0;
        printf "mode=%s\n", mode;
        printf "target=%s:%s\n", host, port;
        printf "requested=%d\n", count;
        printf "concurrency=%d\n", concurrency;
        printf "ok=%d\n", ok;
        printf "fail=%d\n", total_fail;
        printf "elapsed_ms=%d\n", elapsed_ms;
        printf "throughput_ops_per_sec=%.2f\n", throughput;
        printf "latency_avg_ms=%.2f\n", avg;
        printf "latency_p50_ms=%d\n", p50;
        printf "latency_p95_ms=%d\n", p95;
        printf "latency_p99_ms=%d\n", p99;
    }
' "$latencies"
