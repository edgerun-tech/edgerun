#!/usr/bin/env python3
import json
import sys
from pathlib import Path

VALID_STATUSES = {"planned", "in_progress", "completed"}
VALID_SEVERITIES = {"low", "medium", "high", "critical"}
VALID_FINDING_STATUSES = {"open", "closed", "accepted_risk"}


def fail(msg: str) -> None:
    raise SystemExit(f"external-review validation failed: {msg}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate_external_security_review.py <SECURITY_FINDINGS.json>")

    path = Path(sys.argv[1])
    if not path.exists():
        fail(f"file not found: {path}")

    data = json.loads(path.read_text())

    for key in [
        "review_cycle_id",
        "status",
        "provider",
        "scope_version",
        "sign_off",
        "findings",
    ]:
        if key not in data:
            fail(f"missing top-level key: {key}")

    if data["status"] not in VALID_STATUSES:
        fail(f"invalid status: {data['status']}")

    provider = data["provider"]
    if not isinstance(provider, dict):
        fail("provider must be an object")
    for key in ["organization", "reviewer"]:
        if key not in provider:
            fail(f"missing provider.{key}")

    sign_off = data["sign_off"]
    if not isinstance(sign_off, dict):
        fail("sign_off must be an object")
    for key in ["date", "approved", "notes"]:
        if key not in sign_off:
            fail(f"missing sign_off.{key}")

    findings = data["findings"]
    if not isinstance(findings, list):
        fail("findings must be a list")

    unresolved_high_or_critical = []
    for i, finding in enumerate(findings):
        if not isinstance(finding, dict):
            fail(f"finding[{i}] must be an object")
        for key in ["id", "title", "severity", "status", "owner", "notes"]:
            if key not in finding:
                fail(f"finding[{i}] missing key: {key}")
        sev = finding["severity"]
        st = finding["status"]
        if sev not in VALID_SEVERITIES:
            fail(f"finding[{i}] invalid severity: {sev}")
        if st not in VALID_FINDING_STATUSES:
            fail(f"finding[{i}] invalid status: {st}")
        if sev in {"high", "critical"} and st != "closed":
            unresolved_high_or_critical.append(finding["id"])

    if data["status"] == "completed":
        if not sign_off["approved"]:
            fail("completed review requires sign_off.approved=true")
        if not str(sign_off["date"]).strip():
            fail("completed review requires non-empty sign_off.date")
        if not str(provider.get("organization", "")).strip() or provider["organization"] == "TBD":
            fail("completed review requires provider.organization")
        if not str(provider.get("reviewer", "")).strip() or provider["reviewer"] == "TBD":
            fail("completed review requires provider.reviewer")
        if unresolved_high_or_critical:
            fail(
                "completed review cannot have unresolved high/critical findings: "
                + ", ".join(unresolved_high_or_critical)
            )

    print("external-review validation passed")


if __name__ == "__main__":
    main()
