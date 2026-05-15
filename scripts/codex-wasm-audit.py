#!/usr/bin/env python3
"""Audit crates/edgerun-codex for native/wasm split blockers.

This is intentionally static and dependency-free. It gives us a repeatable
check before trying expensive cargo wasm builds.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CODEX = ROOT / "crates" / "edgerun-codex"

FORBIDDEN_ANYWHERE = {
    "v8": "V8 must not exist inside edgerun-codex. Delegate JS execution to Bun/Node/browser worker/EdgeRun runtime capability.",
    "edgerun-v8": "V8 must not exist inside edgerun-codex. Delegate JS execution to Bun/Node/browser worker/EdgeRun runtime capability.",
}

NATIVE_ONLY_DEPS = {
    "edgerun-reqwest",
    "edgerun-tungstenite",
    "edgerun-tungstenite",
    "edgerun-tokio-util",
    "rustls",
    "rustls-native-certs",
    "rustls-pki-types",
    "landlock",
    "seccompiler",
}

HEAVY_DEPS = {
    "tree-sitter",
    "tree-sitter-bash",
    "schemars",
    "ts-rs",
    "icu_decimal",
    "icu_locale_core",
    "icu_provider",
    "tracing-opentelemetry",
    "opentelemetry",
}

NATIVE_PATTERNS = [
    r"std::process",
    r"tokio::process",
    r"std::fs",
    r"tokio::fs",
    r"std::net",
    r"tokio::net",
    r"Command::new",
    r"TcpStream",
    r"TcpListener",
    r"UnixStream",
    r"UnixListener",
]

LIKELY_WASM_FIRST = [
    "app-protocol",
    "protocol",
    "tools",
    "shell",
    "code",
    "info",
    "catalog",
    "debug-context",
]

LIKELY_NATIVE_SPLIT = [
    "api",
    "client",
    "provider",
    "patch",
    "core",
]


def run(cmd: list[str]) -> str:
    result = subprocess.run(cmd, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    return result.stdout


def manifests() -> list[Path]:
    return sorted(CODEX.glob("**/Cargo.toml"))


def package_name(manifest: Path) -> str:
    text = manifest.read_text()
    match = re.search(r"^name\s*=\s*\"([^\"]+)\"", text, re.M)
    return match.group(1) if match else manifest.parent.name


def deps_in_manifest(manifest: Path) -> set[str]:
    deps: set[str] = set()
    for line in manifest.read_text().splitlines():
        match = re.match(r"([A-Za-z0-9_-]+)\s*=", line.strip())
        if match:
            deps.add(match.group(1))
    return deps


def features(manifest: Path) -> dict[str, list[str]]:
    text = manifest.read_text()
    out: dict[str, list[str]] = {}
    in_features = False
    current: str | None = None
    for raw in text.splitlines():
        line = raw.strip()
        if line == "[features]":
            in_features = True
            continue
        if in_features and line.startswith("["):
            break
        if not in_features or not line or line.startswith("#"):
            continue
        match = re.match(r"([A-Za-z0-9_-]+)\s*=\s*\[(.*)", line)
        if match:
            current = match.group(1)
            out[current] = re.findall(r'"([^"]+)"', match.group(2))
            if "]" in line:
                current = None
        elif current:
            out[current].extend(re.findall(r'"([^"]+)"', line))
            if "]" in line:
                current = None
    return out


def native_refs(crate_dir: Path) -> list[tuple[str, int, str]]:
    refs: list[tuple[str, int, str]] = []
    for rs in crate_dir.rglob("*.rs"):
        text = rs.read_text(errors="ignore")
        for i, line in enumerate(text.splitlines(), 1):
            for pat in NATIVE_PATTERNS:
                if re.search(pat, line):
                    refs.append((str(rs.relative_to(ROOT)), i, line.strip()[:160]))
                    break
    return refs


def forbidden_refs() -> list[str]:
    hits: list[str] = []
    for file in CODEX.rglob("*"):
        if not file.is_file() or file.suffix not in {".rs", ".toml", ".lock"}:
            continue
        text = file.read_text(errors="ignore")
        for token, reason in FORBIDDEN_ANYWHERE.items():
            if re.search(rf"\b{re.escape(token)}\b", text):
                hits.append(f"{file.relative_to(ROOT)}: forbidden {token}: {reason}")
    return hits


def print_crate_report() -> None:
    print("Codex wasm/native pruning audit\n")
    print(f"{'crate':30} {'native deps':35} {'heavy deps':35} {'native refs':>11} notes")
    print("-" * 135)
    for manifest in manifests():
        crate = package_name(manifest)
        deps = deps_in_manifest(manifest)
        native_deps = sorted(deps & NATIVE_ONLY_DEPS)
        heavy_deps = sorted(deps & HEAVY_DEPS)
        refs = native_refs(manifest.parent)
        notes: list[str] = []
        feats = features(manifest)
        if "native-transport" in feats:
            notes.append("native-transport gated")
        if crate.replace("codex-", "") in LIKELY_WASM_FIRST or manifest.parent.name in LIKELY_WASM_FIRST:
            notes.append("wasm-first candidate")
        if crate.replace("codex-", "") in LIKELY_NATIVE_SPLIT or manifest.parent.name in LIKELY_NATIVE_SPLIT:
            notes.append("needs native/browser split")
        print(f"{crate:30} {','.join(native_deps)[:35]:35} {','.join(heavy_deps)[:35]:35} {len(refs):11} {', '.join(notes)}")


def print_commands() -> None:
    print("\nSuggested cargo checks:\n")
    for pkg in [
        "codex-app-server-protocol",
        "codex-protocol",
        "codex-tools",
        "codex-shell-command",
        "codex-core",
        "codex-api",
        "codex-client",
        "codex-model-provider",
    ]:
        print(f"cd crates/edgerun-codex && cargo check -p {pkg} --target wasm32-unknown-unknown --no-default-features")


def main() -> None:
    print_crate_report()
    hits = forbidden_refs()
    if hits:
        print("\nForbidden dependencies/references found:\n")
        for hit in hits:
            print(f"- {hit}")
    print_commands()
    print("\nPolicy:\n- V8 must not exist inside edgerun-codex. Delegate JS to Bun/Node/browser worker/EdgeRun runtime capability.\n- wasm crates must not depend on native-transport.\n- reqwest/tokio net/fs/process/TLS/native certs stay behind native features.\n- logging must go through edgerun-log facade, not raw tracing trees.\n- tree-sitter is an optional capability node, not a browser-core dependency.")
    if hits:
        sys.exit(1)


if __name__ == "__main__":
    main()
