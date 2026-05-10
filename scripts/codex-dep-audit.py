#!/usr/bin/env python3
"""Report direct external dependencies left in crates/edgerun-codex."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CODEX = ROOT / "crates" / "edgerun-codex"
CODEX_MANIFEST = CODEX / "Cargo.toml"
UTILITY = ROOT / "crates" / "utility"

ALIASES = {
    "rustls": "edgerun-rusttls",
}

EXISTING_HOME = {
    "anyhow": "edgerun-error exists, but it is a derive crate today; use a compatibility wrapper first unless error call sites are being rewritten.",
    "icu_decimal": "edgerun-locale exists, but Codex uses ICU APIs directly; wrap first.",
    "icu_locale_core": "edgerun-locale exists, but Codex uses ICU APIs directly; wrap first.",
    "icu_provider": "edgerun-locale exists, but Codex uses ICU APIs directly; wrap first.",
    "rustls": "edgerun-rusttls exists but is not API-complete for Codex client tests/custom CA; either extend it or use a temporary compatibility wrapper.",
    "tracing": "edgerun-log exists, but Codex uses tracing spans/macros/instrumentation directly; wrap first.",
    "tracing-opentelemetry": "edgerun-log exists, but OpenTelemetry span propagation still uses upstream APIs; wrap first.",
}

LANES = {
    "anyhow": "broad wrapper",
    "deno_core_icudata": "trivial wrapper",
    "futures": "broad wrapper",
    "icu_decimal": "small wrapper",
    "icu_locale_core": "small wrapper",
    "icu_provider": "small wrapper",
    "landlock": "platform wrapper",
    "opentelemetry": "telemetry wrapper",
    "reqwest": "http client wrapper",
    "rmcp": "protocol wrapper",
    "rustls": "extend existing/wrapper",
    "rustls-native-certs": "tls wrapper",
    "rustls-pki-types": "tls wrapper",
    "schemars": "derive/schema wrapper",
    "seccompiler": "platform wrapper",
    "serde": "derive/serialization wrapper",
    "serde_with": "derive/serialization wrapper",
    "strum": "enum wrapper",
    "strum_macros": "enum derive wrapper",
    "tokio": "runtime wrapper",
    "tokio-util": "runtime wrapper",
    "tracing": "telemetry/log wrapper",
    "tracing-opentelemetry": "telemetry wrapper",
    "tree-sitter": "parser wrapper",
    "tree-sitter-bash": "parser wrapper",
    "ts-rs": "derive/schema wrapper",
    "v8": "runtime wrapper",
}


def workspace_deps() -> list[str]:
    in_deps = False
    deps: list[str] = []
    for line in CODEX_MANIFEST.read_text().splitlines():
        if line.strip() == "[workspace.dependencies]":
            in_deps = True
            continue
        if in_deps and line.startswith("["):
            break
        if in_deps:
            match = re.match(r"([A-Za-z0-9_-]+)\s*=", line)
            if match:
                name = match.group(1)
                if 'package = "edgerun-' in line or 'path = "../utility/edgerun-' in line:
                    continue
                if not name.startswith(("codex-", "edgerun-")):
                    deps.append(name)
    return deps


def rg_count(pattern: str, *globs: str) -> tuple[int, list[str]]:
    command = ["rg", "-l", pattern, str(CODEX), *sum((["-g", glob] for glob in globs), [])]
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    files = sorted({line for line in result.stdout.splitlines() if line})
    return len(files), files


def consumers(dep: str) -> list[str]:
    count, files = rg_count(
        rf"^{re.escape(dep)}\s*=",
        "Cargo.toml",
    )
    _ = count
    return [str(Path(file).relative_to(ROOT)) for file in files if file != str(CODEX_MANIFEST)]


def main() -> None:
    deps = workspace_deps()
    print(f"{len(deps)} direct external workspace dependencies remain\n")
    print(f"{'dependency':24} {'files':>5} {'existing edgerun crate':22} {'lane':22} consumers")
    print("-" * 120)
    for dep in deps:
        crate_ident = dep.replace("-", "_")
        count, _ = rg_count(rf"\b({re.escape(dep)}|{re.escape(crate_ident)})\b", "*.rs", "Cargo.toml")
        wrapper = ALIASES.get(dep, f"edgerun-{dep}")
        existing = "yes" if (UTILITY / wrapper / "Cargo.toml").exists() else "-"
        used_by = ", ".join(consumers(dep))
        print(f"{dep:24} {count:5} {existing:22} {LANES.get(dep, ''):22} {used_by}")
    print("\nExisting Edgerun homes to account for:")
    for dep, note in EXISTING_HOME.items():
        if dep in deps:
            print(f"- {dep}: {note}")


if __name__ == "__main__":
    main()
