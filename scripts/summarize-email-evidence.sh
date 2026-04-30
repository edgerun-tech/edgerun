#!/bin/sh
set -eu

bundle="${1:-}"
if [ -z "$bundle" ] || [ ! -d "$bundle/raw" ]; then
    echo "usage: $0 docs/benchmarks/email-stack/BUNDLE" >&2
    exit 2
fi

raw="$bundle/raw"

value() {
    key="$1"
    file="$2"
    awk -F= -v key="$key" '$1 == key { print $2; found = 1; exit } END { if (!found) print "" }' "$file"
}

memory_summary() {
    file="$1"
    awk '
        function to_mb(value, unit) {
            if (unit == "GB") return value * 1024;
            if (unit == "MB") return value;
            if (unit == "kB") return value / 1024;
            if (unit == "B") return value / 1048576;
            return value;
        }
        NR == 1 { next }
        {
            for (i = 1; i <= NF; i++) {
                if ($i ~ /^mem=/) {
                    raw = substr($i, 5);
                    if (match(raw, /^([0-9.]+)([A-Za-z]+)$/, parts)) {
                        mb = to_mb(parts[1] + 0, parts[2]);
                        if (mb > max_mb) max_mb = mb;
                    }
                }
                if ($i ~ /^pids=/) {
                    pids = substr($i, 6) + 0;
                    if (pids > max_pids) max_pids = pids;
                }
            }
        }
        END {
            if (max_mb > 0 || max_pids > 0) {
                printf "%.2f MB / %d", max_mb, max_pids;
            } else {
                printf "";
            }
        }
    ' "$file"
}

printf '# Email Evidence Summary\n\n'
printf 'Bundle: `%s`\n\n' "$bundle"

printf '## Protocol Results\n\n'
printf '| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |\n'
printf '| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n'
find "$raw" -maxdepth 1 \( -name '*-smtp*.out' -o -name '*-imap*.out' \) \
    ! -name '*warmup.out' -type f -print | sort | while IFS= read -r file; do
    run="$(basename "$file" .out)"
    mode="$(value mode "$file")"
    requested="$(value requested "$file")"
    ok="$(value ok "$file")"
    fail="$(value fail "$file")"
    elapsed="$(value elapsed_ms "$file")"
    throughput="$(value throughput_ops_per_sec "$file")"
    p50="$(value latency_p50_ms "$file")"
    p95="$(value latency_p95_ms "$file")"
    p99="$(value latency_p99_ms "$file")"
    mem=""
    if [ -f "$raw/$run-memory.tsv" ]; then
        mem="$(memory_summary "$raw/$run-memory.tsv")"
    fi
    printf '| `%s` | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
        "$run" "$mode" "$requested" "$ok" "$fail" "$elapsed" "$throughput" "$p50" "$p95" "$p99" "$mem"
done

printf '\n## Stack Shape\n\n'
printf '| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |\n'
printf '| --- | --- | ---: | --- | --- | ---: | --- | --- |\n'
find "$raw" -maxdepth 1 -name '*-metrics.out' -type f -print | sort | while IFS= read -r file; do
    stack="$(value stack "$file")"
    shape="$(value shape "$file")"
    components="$(value components "$file")"
    base="$(value container_base "$file")"
    packages="$(value packages "$file")"
    config_lines="$(value config_lines "$file")"
    services="$(value services "$file")"
    ports="$(value ports "$file")"
    printf '| %s | %s | %s | %s | %s | %s | %s | %s |\n' \
        "$stack" "$shape" "$components" "$base" "$packages" "$config_lines" "$services" "$ports"
done

printf '\n## Versions\n\n'
find "$raw" -maxdepth 1 -name '*-versions.out' -type f -print | sort | while IFS= read -r file; do
    stack="$(basename "$file" -versions.out)"
    printf '### %s\n\n' "$stack"
    printf '```text\n'
    awk '$0 !~ /^\$/ { print }' "$file"
    printf '```\n\n'
done

printf '\n## Images And Timing\n\n'
printf '| Stack | Image bytes | Build ms | Run ms |\n'
printf '| --- | ---: | ---: | ---: |\n'
find "$raw" -maxdepth 1 -name '*-image.out' -type f -print | sort | while IFS= read -r file; do
    stack="$(basename "$file" -image.out)"
    image_bytes="$(awk -F'size=' '$0 !~ /^\$/ && /size=/ { print $2; exit }' "$file")"
    build_ms=""
    run_ms=""
    [ -f "$raw/$stack-build.timing" ] && build_ms="$(value elapsed_ms "$raw/$stack-build.timing")"
    [ -f "$raw/$stack-run.timing" ] && run_ms="$(value elapsed_ms "$raw/$stack-run.timing")"
    printf '| %s | %s | %s | %s |\n' "$stack" "$image_bytes" "$build_ms" "$run_ms"
done
