#!/bin/sh
set -eu

# Collect publishable raw evidence for the email comparison post. This captures
# both protocol measurements and setup/operator-experience data for each stack.
# It binds only localhost ports through the rootless Podman harness.

out="${1:-docs/benchmarks/email-stack/$(date -u '+%Y%m%dTstory-v1')}"
raw="$out/raw"

smtp_count="${EDGERUN_EMAIL_STORY_SMTP_COUNT:-10000}"
imap_count="${EDGERUN_EMAIL_STORY_IMAP_COUNT:-100}"
concurrency="${EDGERUN_EMAIL_STORY_CONCURRENCY:-32}"
edgerun_bin="${EDGERUN_EMAIL_STORY_EDGERUN_BIN:-}"
smtp_bench_bin="${EDGERUN_EMAIL_STORY_SMTP_BENCH_BIN:-${EDGERUN_EMAIL_BENCH_SMTP_BENCH_BIN:-}}"
protocol_timeout="${EDGERUN_EMAIL_STORY_PROTOCOL_TIMEOUT:-120}"
setup_timeout="${EDGERUN_EMAIL_STORY_SETUP_TIMEOUT:-300}"
include_native_edgerun="${EDGERUN_EMAIL_STORY_INCLUDE_NATIVE_EDGERUN:-0}"
stacks="${EDGERUN_EMAIL_STORY_STACKS:-edgerun postfix opensmtpd dovecot maddy mox stalwart}"
repetitions="${EDGERUN_EMAIL_STORY_REPETITIONS:-1}"
smtp_warmup_count="${EDGERUN_EMAIL_STORY_SMTP_WARMUP_COUNT:-0}"
imap_warmup_count="${EDGERUN_EMAIL_STORY_IMAP_WARMUP_COUNT:-0}"
allow_long_timeout="${EDGERUN_EMAIL_STORY_ALLOW_LONG_TIMEOUT:-0}"

case "$protocol_timeout" in
    ''|*[!0-9]*) echo "EDGERUN_EMAIL_STORY_PROTOCOL_TIMEOUT must be numeric" >&2; exit 2 ;;
esac
case "$setup_timeout" in
    ''|*[!0-9]*) echo "EDGERUN_EMAIL_STORY_SETUP_TIMEOUT must be numeric" >&2; exit 2 ;;
esac
if [ "$protocol_timeout" -gt 300 ] && [ "$allow_long_timeout" != "1" ]; then
    echo "refusing protocol timeout above 300s; set EDGERUN_EMAIL_STORY_ALLOW_LONG_TIMEOUT=1 for intentionally long runs" >&2
    exit 2
fi

mkdir -p "$raw"

now_ms() {
    printf '%s\n' "$(($(date +%s%N) / 1000000))"
}

write_command_header() {
    printf '$'
    for arg in "$@"; do
        printf ' %s' "$arg"
    done
    printf '\n'
}

run_capture() {
    name="$1"
    shift
    {
        write_command_header "$@"
        "$@"
    } > "$raw/$name.out" 2> "$raw/$name.err" || {
        status="$?"
        printf 'exit_status=%s\n' "$status" >> "$raw/$name.out"
        return 0
    }
}

run_timed_capture() {
    name="$1"
    shift
    status=0
    start="$(now_ms)"
    {
        write_command_header timeout "$setup_timeout" "$@"
        timeout "$setup_timeout" "$@"
    } > "$raw/$name.out" 2> "$raw/$name.err" || status="$?"
    end="$(now_ms)"
    status="${status:-0}"
    {
        printf 'command='
        for arg in "$@"; do
            printf '%s ' "$arg"
        done
        printf '\nstatus=%s\nstarted_ms=%s\nended_ms=%s\nelapsed_ms=%s\n' \
            "$status" "$start" "$end" "$((end - start))"
    } > "$raw/$name.timing"
    return 0
}

sample_podman_until() {
    stack="$1"
    pid="$2"
    file="$3"
    name="edgerun-email-bench-$stack"
    printf 'timestamp_ms stats\n' > "$file"
    while kill -0 "$pid" 2>/dev/null; do
        printf '%s ' "$(now_ms)" >> "$file"
        podman stats --no-stream --format \
            'name={{.Name}} cpu={{.CPUPerc}} mem={{.MemUsage}} net={{.NetIO}} block={{.BlockIO}} pids={{.PIDs}}' \
            "$name" >> "$file" 2>/dev/null || printf 'stats_unavailable\n' >> "$file"
        sleep 0.5
    done
}

sample_process_until() {
    target_pid="$1"
    bench_pid="$2"
    file="$3"
    printf 'timestamp_ms rss_kb vsz_kb\n' > "$file"
    while kill -0 "$bench_pid" 2>/dev/null; do
        printf '%s ' "$(now_ms)" >> "$file"
        ps -o rss=,vsz= -p "$target_pid" 2>/dev/null | awk '{ print $1, $2 }' >> "$file" || printf 'process_unavailable\n' >> "$file"
        sleep 0.5
    done
}

run_sampled_process_protocol() {
    target_pid="$1"
    name="$2"
    shift 2
    {
        write_command_header timeout "$protocol_timeout" "$@"
        timeout "$protocol_timeout" "$@"
    } > "$raw/$name.out" 2> "$raw/$name.err" &
    bench_pid="$!"
    sample_process_until "$target_pid" "$bench_pid" "$raw/$name-memory.tsv" &
    sample_pid="$!"
    status=0
    wait "$bench_pid" || status="$?"
    if [ "$status" != "0" ]; then
        printf 'exit_status=%s\n' "$status" >> "$raw/$name.out"
    fi
    wait "$sample_pid" 2>/dev/null || true
}

wait_tcp() {
    host="$1"
    port="$2"
    deadline="$(( $(now_ms) + 10000 ))"
    while [ "$(now_ms)" -lt "$deadline" ]; do
        if bash -c "exec 3<>/dev/tcp/$host/$port" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.1
    done
    return 1
}

run_sampled_protocol() {
    stack="$1"
    name="$2"
    shift 2
    {
        write_command_header timeout "$protocol_timeout" "$@"
        timeout "$protocol_timeout" "$@"
    } > "$raw/$name.out" 2> "$raw/$name.err" &
    bench_pid="$!"
    sample_podman_until "$stack" "$bench_pid" "$raw/$name-memory.tsv" &
    sample_pid="$!"
    status=0
    wait "$bench_pid" || status="$?"
    if [ "$status" != "0" ]; then
        printf 'exit_status=%s\n' "$status" >> "$raw/$name.out"
    fi
    wait "$sample_pid" 2>/dev/null || true
}

run_protocol_repetitions() {
    stack="$1"
    base="$2"
    workload="$3"
    count="$4"
    warmup_count="$5"
    stack_concurrency="$concurrency"
    if [ "$stack" = "mox" ]; then
        stack_concurrency="${EDGERUN_EMAIL_STORY_MOX_CONCURRENCY:-8}"
    elif [ "$stack" = "stalwart" ]; then
        stack_concurrency="${EDGERUN_EMAIL_STORY_STALWART_CONCURRENCY:-1}"
    fi
    if [ "$warmup_count" != "0" ]; then
        run_capture "$base-warmup" \
            timeout "$protocol_timeout" \
            env EDGERUN_EMAIL_BENCH_COUNT="$warmup_count" \
                EDGERUN_EMAIL_BENCH_CONCURRENCY="$stack_concurrency" \
                ./scripts/benchmark-email-podman.sh "$workload" "$stack"
    fi
    rep=1
    while [ "$rep" -le "$repetitions" ]; do
        if [ "$repetitions" = "1" ]; then
            name="$base"
        else
            name="$base-r$rep"
        fi
        run_sampled_protocol "$stack" "$name" \
            env EDGERUN_EMAIL_BENCH_COUNT="$count" \
                EDGERUN_EMAIL_BENCH_CONCURRENCY="$stack_concurrency" \
                ./scripts/benchmark-email-podman.sh "$workload" "$stack"
        rep=$((rep + 1))
    done
}

stack_kind() {
    case "$1" in
        edgerun) printf 'smtp_imap' ;;
        maddy) printf 'smtp_imap' ;;
        dovecot) printf 'imap' ;;
        mox) printf 'smtp_imap' ;;
        stalwart) printf 'smtp_imap' ;;
        *) printf 'smtp' ;;
    esac
}

collect_stack() {
    stack="$1"
    kind="$(stack_kind "$stack")"
    stack_smtp_count="$smtp_count"
    stack_imap_count="$imap_count"
    if [ "$stack" = "mox" ]; then
        stack_smtp_count="${EDGERUN_EMAIL_STORY_MOX_SMTP_COUNT:-100}"
        stack_imap_count="${EDGERUN_EMAIL_STORY_MOX_IMAP_COUNT:-100}"
    elif [ "$stack" = "stalwart" ]; then
        stack_smtp_count="${EDGERUN_EMAIL_STORY_STALWART_SMTP_COUNT:-10}"
        stack_imap_count="${EDGERUN_EMAIL_STORY_STALWART_IMAP_COUNT:-100}"
    fi
    run_capture "$stack-notes" ./scripts/benchmark-email-podman.sh notes "$stack"
    run_capture "$stack-config" ./scripts/benchmark-email-podman.sh config "$stack"
    run_capture "$stack-metrics" ./scripts/benchmark-email-podman.sh metrics "$stack"
    run_timed_capture "$stack-build" ./scripts/benchmark-email-podman.sh build "$stack"
    run_capture "$stack-versions" ./scripts/benchmark-email-podman.sh versions "$stack"
    run_capture "$stack-image" podman image inspect "localhost/edgerun-email-bench-$stack:latest" \
        --format 'id={{.Id}} created={{.Created}} size={{.Size}}'
    run_timed_capture "$stack-run" ./scripts/benchmark-email-podman.sh run "$stack"
    run_capture "$stack-ps" podman ps --filter "name=edgerun-email-bench-$stack" \
        --format '{{.Names}} {{.Status}} {{.Ports}}'
    run_capture "$stack-stats-before" podman stats --no-stream --format \
        'name={{.Name}} cpu={{.CPUPerc}} mem={{.MemUsage}} net={{.NetIO}} block={{.BlockIO}} pids={{.PIDs}}' \
        "edgerun-email-bench-$stack"

    case "$kind" in
        smtp_imap)
            run_protocol_repetitions "$stack" "$stack-smtp" bench-smtp "$stack_smtp_count" "$smtp_warmup_count"
            run_protocol_repetitions "$stack" "$stack-imap" bench-imap "$stack_imap_count" "$imap_warmup_count"
            ;;
        smtp)
            run_protocol_repetitions "$stack" "$stack-smtp" bench-smtp "$smtp_count" "$smtp_warmup_count"
            ;;
        imap)
            run_protocol_repetitions "$stack" "$stack-imap" bench-imap "$imap_count" "$imap_warmup_count"
            ;;
        bootstrap)
            run_capture "$stack-logs-bootstrap" podman logs --tail 240 "edgerun-email-bench-$stack"
            ;;
    esac

    run_capture "$stack-stats-after" podman stats --no-stream --format \
        'name={{.Name}} cpu={{.CPUPerc}} mem={{.MemUsage}} net={{.NetIO}} block={{.BlockIO}} pids={{.PIDs}}' \
        "edgerun-email-bench-$stack"
    run_capture "$stack-logs" podman logs --tail 240 "edgerun-email-bench-$stack"
    run_timed_capture "$stack-stop" ./scripts/benchmark-email-podman.sh stop "$stack"
}

find_edgerun_bin() {
    if [ -n "$edgerun_bin" ]; then
        printf '%s\n' "$edgerun_bin"
    elif command -v edgerun-server >/dev/null 2>&1; then
        command -v edgerun-server
    elif [ -x target/x86_64-unknown-linux-musl/release/edgerun-server ]; then
        printf '%s\n' "target/x86_64-unknown-linux-musl/release/edgerun-server"
    elif [ -x target/release/edgerun-server ]; then
        printf '%s\n' "target/release/edgerun-server"
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
    fi
}

collect_native_edgerun_snapshot() {
    bin="$(find_edgerun_bin || true)"
    if [ -z "$bin" ]; then
        printf 'edgerun-server binary not found; build edgerun-server before collecting native Edgerun snapshot\n' > "$raw/native-edgerun-skipped.out"
        return
    fi
    work="$out/native-edgerun-work"
    maildir="$work/maildirs"
    queue="$work/queue"
    config="$work/server.yaml"
    mkdir -p "$maildir" "$queue"
    cat > "$config" <<EOF
apiVersion: edgerun.io/v1alpha1
kind: SmtpServer
metadata:
  name: benchmark
spec:
  hostname: benchmark.local
  bind_address: "127.0.0.1:2526"
  smtps: false
  starttls: false
  local_domains:
    - example.test
  maildir_root: $maildir
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
  bind_address: "127.0.0.1:1144"
  imaps: false
  maildir_root: $maildir
  users:
    - username: bench
      password: bench
EOF
    cp "$config" "$raw/native-edgerun-config.out"
    {
        printf 'stack=edgerun\n'
        printf 'shape=integrated_reference_server\n'
        printf 'components=1\n'
        printf 'binary=%s\n' "$bin"
        stat -c 'binary_bytes=%s' "$bin" 2>/dev/null || true
        printf 'config_lines=%s\n' "$(awk 'NF && $1 !~ /^#/ { n++ } END { print n + 0 }' "$config")"
        printf 'services=smtp,imap\n'
        printf 'ports=smtp:2526,imap:1144\n'
        printf 'multitenancy_note=single process with explicit local domains and per-user mailbox roots; tenant policy should become a first-class config dimension before broad hosting claims.\n'
    } > "$raw/native-edgerun-metrics.out"

    start="$(now_ms)"
    "$bin" --config "$config" \
        --webmail-http-bind 127.0.0.1:18081 \
        --webmail-https-bind 127.0.0.1:18443 \
        > "$raw/native-edgerun-server.out" 2> "$raw/native-edgerun-server.err" &
    server_pid="$!"
    ready_status=0
    wait_tcp 127.0.0.1 2526 || ready_status="$?"
    wait_tcp 127.0.0.1 1144 || ready_status="$?"
    end="$(now_ms)"
    {
        printf 'command=%s --config %s --webmail-http-bind 127.0.0.1:18081 --webmail-https-bind 127.0.0.1:18443\n' "$bin" "$config"
        printf 'status=%s\nstarted_ms=%s\nended_ms=%s\nelapsed_ms=%s\npid=%s\n' \
            "$ready_status" "$start" "$end" "$((end - start))" "$server_pid"
    } > "$raw/native-edgerun-run.timing"

    if [ "$ready_status" = "0" ]; then
        smtp_bench="$(find_smtp_bench_bin || true)"
        if [ -n "$smtp_bench" ]; then
            run_sampled_process_protocol "$server_pid" native-edgerun-smtp \
                "$smtp_bench" \
                    --host 127.0.0.1 \
                    --port 2526 \
                    --count "$smtp_count" \
                    --concurrency "$concurrency" \
                    --from bench@example.test \
                    --to bench@example.test
        else
            run_sampled_process_protocol "$server_pid" native-edgerun-smtp \
                ./scripts/benchmark-email-protocol.sh smtp \
                    --host 127.0.0.1 \
                    --port 2526 \
                    --count "$smtp_count" \
                    --concurrency "$concurrency" \
                    --from bench@example.test \
                    --to bench@example.test
        fi
        run_sampled_process_protocol "$server_pid" native-edgerun-imap \
            ./scripts/benchmark-email-protocol.sh imap \
                --host 127.0.0.1 \
                --port 1144 \
                --count "$imap_count" \
                --concurrency "$concurrency" \
                --user bench \
                --password bench
    fi

    ps -o pid=,ppid=,comm=,rss=,vsz=,etime=,args= -p "$server_pid" > "$raw/native-edgerun-ps.out" 2> "$raw/native-edgerun-ps.err" || true
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
}

run_capture host-uname uname -a
run_capture host-os sh -c '. /etc/os-release && printf "%s %s\n" "$NAME" "$VERSION_ID"'
run_capture podman-version podman --version
run_capture podman-info podman info --format 'rootless={{.Host.Security.Rootless}} graph={{.Store.GraphRoot}}'
run_capture images-before podman images --format '{{.Repository}}:{{.Tag}} {{.ID}} {{.Size}}'

if [ -x ./scripts/benchmark-email-stack.sh ]; then
    run_capture edgerun-live-stack ./scripts/benchmark-email-stack.sh --local
fi

if [ "$include_native_edgerun" = "1" ]; then
    collect_native_edgerun_snapshot
fi

for stack in $stacks; do
    collect_stack "$stack"
done

run_capture images-after podman images --format '{{.Repository}}:{{.Tag}} {{.ID}} {{.Size}}'

cat > "$out/README.md" <<EOF
# Email Stack Comparison Evidence

Captured at: $(date -u '+%Y-%m-%dT%H:%M:%SZ')

This bundle is raw evidence for a comparison post. It intentionally records
operator-experience data before the prose is written:

- generated benchmark config for each stack,
- stack/component metrics from the harness,
- package/server versions from each built image,
- image footprint before and after the run,
- build time through the rootless Podman harness,
- run/start time through the rootless Podman harness,
- one-at-a-time protocol benchmark output,
- memory/process samples while protocol load is running,
- logs and final stats.

Workload:

- Edgerun rootless Podman: $smtp_count local-only SMTP loopback deliveries and
  $imap_count IMAP login/list/select/logout sessions, concurrency $concurrency.
- SMTP-only stacks: $smtp_count local-only loopback deliveries, concurrency
  $concurrency, recipient bench@example.test, no external delivery.
- Dovecot IMAP: $imap_count login/list/select/logout sessions, concurrency
  $concurrency, benchmark user bench.
- Protocol commands have a $protocol_timeout second wall-clock cap. A timeout is
  recorded as exit status 124 in the corresponding raw output.
- Setup commands have a $setup_timeout second wall-clock cap.
- Protocol timeouts above 300 seconds are refused unless
  EDGERUN_EMAIL_STORY_ALLOW_LONG_TIMEOUT=1 is set.
- Measured repetitions per workload: $repetitions.
- SMTP warmup operations before measured repetitions: $smtp_warmup_count.
- IMAP warmup operations before measured repetitions: $imap_warmup_count.
- Mox: localserve SMTP/IMAP evidence. This is local development/test mode, not
  production quickstart evidence. Defaults are
  ${EDGERUN_EMAIL_STORY_MOX_SMTP_COUNT:-100} SMTP operations,
  ${EDGERUN_EMAIL_STORY_MOX_IMAP_COUNT:-100} IMAP sessions, concurrency
  ${EDGERUN_EMAIL_STORY_MOX_CONCURRENCY:-8}.
- Stalwart: automated v0.16 bootstrap/setup evidence plus SMTP/IMAP smoke.
  Defaults are
  ${EDGERUN_EMAIL_STORY_STALWART_SMTP_COUNT:-10} SMTP operations,
  ${EDGERUN_EMAIL_STORY_STALWART_IMAP_COUNT:-100} IMAP sessions, concurrency
  ${EDGERUN_EMAIL_STORY_STALWART_CONCURRENCY:-2}.
- Native Edgerun snapshot: disabled by default. Set
  EDGERUN_EMAIL_STORY_INCLUDE_NATIVE_EDGERUN=1 to collect it as production-shape
  evidence, not as a direct performance comparator. Native SMTP uses
  \`edgerun-smtp-bench\` when available and the shell protocol benchmark as a
  fallback.
- Stack selection: \`$stacks\`. Override with EDGERUN_EMAIL_STORY_STACKS to run
  one pair or one stack at a time.

All command stdout/stderr files are under \`raw/\`. Timing metadata is stored in
\`raw/*.timing\`.
EOF

printf '%s\n' "$out"
