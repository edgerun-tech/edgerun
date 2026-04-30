#!/bin/sh
set -eu

# Collect reproducible evidence for the email benchmark post. This runs each
# comparison stack one at a time through the rootless Podman harness and writes
# raw outputs into a timestamped directory.
#
# Usage:
#   scripts/collect-email-benchmark-evidence.sh [OUT_DIR]

out="${1:-docs/benchmarks/email-stack/$(date -u '+%Y%m%dT%H%M%SZ')}"
mkdir -p "$out/raw" "$out/logs"

count="${EDGERUN_EMAIL_BENCH_COUNT:-100}"
concurrency="${EDGERUN_EMAIL_BENCH_CONCURRENCY:-4}"
export EDGERUN_EMAIL_BENCH_COUNT="$count"
export EDGERUN_EMAIL_BENCH_CONCURRENCY="$concurrency"

run_capture() {
    name="$1"
    shift
    {
        printf '$'
        for arg in "$@"; do
            printf ' %s' "$arg"
        done
        printf '\n'
        "$@"
    } > "$out/raw/$name.out" 2> "$out/raw/$name.err" || {
        status="$?"
        printf 'exit_status=%s\n' "$status" >> "$out/raw/$name.out"
        return 0
    }
}

run_capture host-uname uname -a
run_capture host-os sh -c '. /etc/os-release && printf "%s %s\n" "$NAME" "$VERSION_ID"'
run_capture podman-version podman --version
run_capture podman-info podman info --format 'rootless={{.Host.Security.Rootless}} graph={{.Store.GraphRoot}}'
run_capture images podman images --format '{{.Repository}}:{{.Tag}} {{.ID}} {{.Size}}'

if [ -x ./scripts/benchmark-email-stack.sh ]; then
    run_capture edgerun-live-stack ./scripts/benchmark-email-stack.sh --local
fi

for stack in postfix exim opensmtpd dovecot stalwart; do
    run_capture "$stack-notes" ./scripts/benchmark-email-podman.sh notes "$stack"
    run_capture "$stack-image" podman image inspect "localhost/edgerun-email-bench-$stack:latest" \
        --format '{{.Id}} {{.Created}} {{.Size}}'

    case "$stack" in
        dovecot)
            run_capture "$stack-run" ./scripts/benchmark-email-podman.sh run "$stack"
            sleep 2
            run_capture "$stack-ps" podman ps --filter "name=edgerun-email-bench-$stack" \
                --format '{{.Names}} {{.Status}} {{.Ports}}'
            run_capture "$stack-stats" podman stats --no-stream --format \
                'name={{.Name}} cpu={{.CPUPerc}} mem={{.MemUsage}} net={{.NetIO}} block={{.BlockIO}} pids={{.PIDs}}'
            run_capture "$stack-imap" ./scripts/benchmark-email-podman.sh bench-imap "$stack"
            run_capture "$stack-logs" podman logs --tail 200 "edgerun-email-bench-$stack"
            run_capture "$stack-stop" ./scripts/benchmark-email-podman.sh stop "$stack"
            ;;
        stalwart)
            run_capture "$stack-run" ./scripts/benchmark-email-podman.sh run "$stack"
            sleep 5
            run_capture "$stack-ps" podman ps --filter "name=edgerun-email-bench-$stack" \
                --format '{{.Names}} {{.Status}} {{.Ports}}'
            run_capture "$stack-stats" podman stats --no-stream --format \
                'name={{.Name}} cpu={{.CPUPerc}} mem={{.MemUsage}} net={{.NetIO}} block={{.BlockIO}} pids={{.PIDs}}'
            run_capture "$stack-logs" podman logs --tail 200 "edgerun-email-bench-$stack"
            run_capture "$stack-stop" ./scripts/benchmark-email-podman.sh stop "$stack"
            ;;
        *)
            run_capture "$stack-run" ./scripts/benchmark-email-podman.sh run "$stack"
            sleep 2
            run_capture "$stack-ps" podman ps --filter "name=edgerun-email-bench-$stack" \
                --format '{{.Names}} {{.Status}} {{.Ports}}'
            run_capture "$stack-stats" podman stats --no-stream --format \
                'name={{.Name}} cpu={{.CPUPerc}} mem={{.MemUsage}} net={{.NetIO}} block={{.BlockIO}} pids={{.PIDs}}'
            run_capture "$stack-smtp" ./scripts/benchmark-email-podman.sh bench-smtp "$stack"
            run_capture "$stack-logs" podman logs --tail 200 "edgerun-email-bench-$stack"
            run_capture "$stack-stop" ./scripts/benchmark-email-podman.sh stop "$stack"
            ;;
    esac
done

cat > "$out/README.md" <<EOF
# Email Stack Benchmark Evidence

Captured at: $(date -u '+%Y-%m-%dT%H:%M:%SZ')

Protocol workload:

- SMTP: local delivery only, localhost high port 2525, $count messages,
  concurrency $concurrency, recipient bench@example.test.
- IMAP: localhost high port 1143, $count login/list/select/logout sessions,
  concurrency $concurrency, user bench.
- Stalwart: image/startup evidence only in this capture until first-run
  domain/account/anti-relay configuration is automated.

All raw command outputs are under \`raw/\`. stderr is stored next to stdout with
the same base name and an \`.err\` suffix.
EOF

printf '%s\n' "$out"
