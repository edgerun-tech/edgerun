#!/usr/bin/env python3
"""Run current benchmark entry points and save a timestamped report."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Command:
    label: str
    argv: list[str]


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        default="target/benchmarks",
        help="directory for benchmark reports",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="list benchmark entry points without running them",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print benchmark commands without running them",
    )
    return parser.parse_args()


def cargo_metadata(root: Path) -> dict:
    proc = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return json.loads(proc.stdout)


def cargo_bench_commands(metadata: dict) -> list[Command]:
    commands: list[Command] = []
    for package in metadata["packages"]:
        bench_names = sorted(
            target["name"]
            for target in package.get("targets", [])
            if "bench" in target.get("kind", [])
        )
        for bench in bench_names:
            commands.append(
                Command(
                    label=f"{package['name']}::{bench}",
                    argv=["cargo", "bench", "-p", package["name"], "--bench", bench],
                )
            )
    return commands


def fallback_commands() -> list[Command]:
    return [
        Command(
            label="edgerun-mesh benchmark tests",
            argv=[
                "cargo",
                "test",
                "-p",
                "edgerun-mesh",
                "benchmark",
                "--",
                "--nocapture",
                "--test-threads=1",
            ],
        )
    ]


def run_and_tee(root: Path, report: Path, command: Command) -> int:
    header = f"\n=== {command.label} ===\n$ {' '.join(command.argv)}\n"
    print(header, end="")
    with report.open("a", encoding="utf-8") as file:
        file.write(header)
        proc = subprocess.Popen(
            command.argv,
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            bufsize=1,
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            print(line, end="")
            file.write(line)
        return proc.wait()


def main() -> int:
    args = parse_args()
    root = repo_root()
    commands = cargo_bench_commands(cargo_metadata(root))
    if not commands:
        commands = fallback_commands()

    if args.list:
        for command in commands:
            print(f"{command.label}\t{' '.join(command.argv)}")
        return 0

    if args.dry_run:
        for command in commands:
            print(f"$ {' '.join(command.argv)}")
        return 0

    out_dir = root / args.output_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    report = out_dir / f"benchmark-{stamp}.txt"

    return_code = 0
    for command in commands:
        code = run_and_tee(root, report, command)
        if code != 0 and return_code == 0:
            return_code = code

    print(f"\nBenchmark report: {report}")
    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
