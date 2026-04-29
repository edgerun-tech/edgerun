#!/usr/bin/env python3
"""Run cargo tests and print an aggregate libtest summary."""

from __future__ import annotations

import argparse
import re
import subprocess
from collections import deque
from dataclasses import dataclass


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


@dataclass
class TestTotals:
    ok_lines: int = 0
    passed: int = 0
    failed: int = 0
    ignored: int = 0
    measured: int = 0
    filtered: int = 0
    failed_tests: list[str] | None = None
    recent_lines: deque[str] | None = None

    def __post_init__(self) -> None:
        if self.failed_tests is None:
            self.failed_tests = []
        if self.recent_lines is None:
            self.recent_lines = deque(maxlen=80)


def run_and_collect(command: list[str], totals: TestTotals) -> int:
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
        totals.recent_lines.append(stripped)
        match = TEST_RESULT_RE.match(stripped)
        if match:
            name = match.group("name")
            status = match.group("status")
            if status == "ok":
                totals.ok_lines += 1
            elif status == "FAILED":
                totals.failed_tests.append(name)
            continue

        summary = TEST_SUMMARY_RE.match(stripped)
        if summary:
            totals.passed += int(summary.group("passed"))
            totals.failed += int(summary.group("failed"))
            totals.ignored += int(summary.group("ignored"))
            totals.measured += int(summary.group("measured"))
            totals.filtered += int(summary.group("filtered"))

    return proc.wait()


def main() -> int:
    args = parse_args()
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    commands = [command] if command else [
        ["cargo", "test", "--workspace"],
        ["cargo", "test", "--manifest-path", "programs/deployment/Cargo.toml"],
        ["cargo", "test", "--manifest-path", "programs/provider_registry/Cargo.toml"],
    ]

    totals = TestTotals()
    return_code = 0
    for idx, cmd in enumerate(commands):
        if idx:
            print()
        code = run_and_collect(cmd, totals)
        if code != 0 and return_code == 0:
            return_code = code

    print("\n=== Test Summary ===")
    print(f"Successful: {totals.passed}")
    print(f"Failed: {totals.failed}")
    print(f"Ignored: {totals.ignored}")
    print(f"Measured: {totals.measured}")
    print(f"Filtered out: {totals.filtered}")
    print(f"Individual ok lines seen: {totals.ok_lines}")

    if totals.failed_tests:
        print("\nFailed tests:")
        for name in totals.failed_tests:
            print(f"  - {name}")
    elif return_code != 0:
        print("\nNo individual failed test lines were seen.")
        print("Cargo likely failed during compile, link, or test harness setup.")
        print("\nRecent output:")
        for line in totals.recent_lines:
            print(line)
    else:
        print("\nFailed tests: none")

    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
