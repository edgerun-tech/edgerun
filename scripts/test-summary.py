#!/usr/bin/env python3
"""Run cargo tests and print an aggregate libtest summary."""

from __future__ import annotations

import argparse
import re
import subprocess
from collections import deque


TEST_RESULT_RE = re.compile(r"^test (?P<name>.+) \.\.\. (?P<status>ok|FAILED|ignored)$")
TEST_SUMMARY_RE = re.compile(
    r"^test result: (?P<status>ok|FAILED)\. "
    r"(?P<passed>\d+) passed; "
    r"(?P<failed>\d+) failed; "
    r"(?P<ignored>\d+) ignored; "
    r"(?P<measured>\d+) measured; "
    r"(?P<filtered>\d+) filtered out;"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run tests, stream output, and summarize pass/fail counts."
    )
    parser.add_argument(
        "command",
        nargs=argparse.REMAINDER,
        help="test command to run; defaults to cargo test --workspace",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        command = ["cargo", "test", "--workspace"]

    ok_lines = 0
    passed_total = 0
    failed_total = 0
    ignored_total = 0
    measured_total = 0
    filtered_total = 0
    failed: list[str] = []
    recent_lines: deque[str] = deque(maxlen=80)

    print(f"Running: {' '.join(command)}", flush=True)
    proc = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    assert proc.stdout is not None

    for line in proc.stdout:
        print(line, end="")
        stripped = line.rstrip("\n")
        recent_lines.append(stripped)
        match = TEST_RESULT_RE.match(stripped)
        if match:
            name = match.group("name")
            status = match.group("status")
            if status == "ok":
                ok_lines += 1
            elif status == "FAILED":
                failed.append(name)
            continue

        summary = TEST_SUMMARY_RE.match(stripped)
        if summary:
            passed_total += int(summary.group("passed"))
            failed_total += int(summary.group("failed"))
            ignored_total += int(summary.group("ignored"))
            measured_total += int(summary.group("measured"))
            filtered_total += int(summary.group("filtered"))

    return_code = proc.wait()

    print("\n=== Test Summary ===")
    print(f"Successful: {passed_total}")
    print(f"Failed: {failed_total}")
    print(f"Ignored: {ignored_total}")
    print(f"Measured: {measured_total}")
    print(f"Filtered out: {filtered_total}")
    print(f"Individual ok lines seen: {ok_lines}")

    if failed:
        print("\nFailed tests:")
        for name in failed:
            print(f"  - {name}")
    elif return_code != 0:
        print("\nNo individual failed test lines were seen.")
        print("Cargo likely failed during compile, link, or test harness setup.")
        print("\nRecent output:")
        for line in recent_lines:
            print(line)
    else:
        print("\nFailed tests: none")

    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
