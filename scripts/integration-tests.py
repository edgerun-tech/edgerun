#!/usr/bin/env python3
"""Run current Cargo integration test targets.

By default this runs integration tests that do not require external hardware.
Hardware-dependent tests stay opt-in behind --hardware or their native
environment gate.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class IntegrationTarget:
    package: str
    name: str
    src_path: str


HARDWARE_TARGETS = {
    ("edgerun-quectel-ec200a", "end_to_end"),
}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--hardware",
        action="store_true",
        help="include hardware-dependent integration tests",
    )
    parser.add_argument(
        "--package",
        "-p",
        action="append",
        default=[],
        help="limit to one package; can be repeated",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="list selected integration test targets without running them",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print cargo commands without running them",
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


def integration_targets(metadata: dict) -> list[IntegrationTarget]:
    targets: list[IntegrationTarget] = []
    for package in metadata["packages"]:
        for target in package.get("targets", []):
            if "test" in target.get("kind", []):
                targets.append(
                    IntegrationTarget(
                        package=package["name"],
                        name=target["name"],
                        src_path=target["src_path"],
                    )
                )
    return sorted(targets, key=lambda item: (item.package, item.name))


def should_run(target: IntegrationTarget, args: argparse.Namespace) -> bool:
    packages = set(args.package)
    if packages and target.package not in packages:
        return False
    if (target.package, target.name) in HARDWARE_TARGETS:
        return args.hardware or os.environ.get("EC200A_E2E") == "1"
    return True


def cargo_test_command(target: IntegrationTarget) -> list[str]:
    return ["cargo", "test", "-p", target.package, "--test", target.name]


def main() -> int:
    args = parse_args()
    root = repo_root()
    targets = integration_targets(cargo_metadata(root))
    selected = [target for target in targets if should_run(target, args)]

    if not selected:
        print("No integration test targets selected.")
        return 0

    if args.list:
        for target in selected:
            print(f"{target.package}::{target.name}\t{target.src_path}")
        return 0

    skipped = [
        target
        for target in targets
        if (target.package, target.name) in HARDWARE_TARGETS
        and not (args.hardware or os.environ.get("EC200A_E2E") == "1")
        and (not args.package or target.package in set(args.package))
    ]
    if skipped:
        print("Skipping hardware-dependent integration targets:")
        for target in skipped:
            print(f"  - {target.package}::{target.name}")
        print(flush=True)

    failures: list[IntegrationTarget] = []
    for target in selected:
        print(f"Running {target.package}::{target.name}", flush=True)
        command = cargo_test_command(target)
        if args.dry_run:
            print(f"  $ {' '.join(command)}")
            continue
        code = subprocess.run(command, cwd=root).returncode
        if code != 0:
            failures.append(target)

    if args.dry_run:
        print("\nDry run complete.")
        return 0

    if failures:
        print("\nFailed integration targets:", file=sys.stderr)
        for target in failures:
            print(f"  - {target.package}::{target.name}", file=sys.stderr)
        return 1

    print("\nIntegration tests passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
