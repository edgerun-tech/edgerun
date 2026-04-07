#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / 'edgerun_core_protocol_v0_single_file.md'
PROTO_DIR = ROOT / 'proto' / 'edgerun' / 'v0'

HEADING_RE = re.compile(r'^### `edgerun/v0/([^`]+\.proto)`\n\n```proto\n(.*?)\n```', re.S | re.M)
MESSAGE_RE = re.compile(r'(?ms)^message\s+(\w+)\s*\{(.*?)^\}')
ENUM_RE = re.compile(r'(?ms)^enum\s+(\w+)\s*\{(.*?)^\}')
FIELD_LINE_RE = re.compile(r'^\s*(?:optional\s+)?(?:repeated\s+)?[A-Za-z0-9_\.<>]+\s+[A-Za-z0-9_]+\s*=\s*\d+\s*;$')
ENUM_VALUE_RE = re.compile(r'^\s*[A-Z0-9_]+\s*=\s*\d+\s*;$')


def norm_line(line: str) -> str:
    return ' '.join(line.strip().split())


def parse_messages(text: str) -> dict[str, set[str]]:
    out: dict[str, set[str]] = {}
    for name, body in MESSAGE_RE.findall(text):
        fields = {
            norm_line(line)
            for line in body.splitlines()
            if FIELD_LINE_RE.match(line)
        }
        out[name] = fields
    return out


def parse_enums(text: str) -> dict[str, set[str]]:
    out: dict[str, set[str]] = {}
    for name, body in ENUM_RE.findall(text):
        values = {
            norm_line(line)
            for line in body.splitlines()
            if ENUM_VALUE_RE.match(line)
        }
        out[name] = values
    return out


def main() -> int:
    spec_text = SPEC.read_text()
    spec_files = {name: body for name, body in HEADING_RE.findall(spec_text)}
    actual_files = {p.name: p.read_text() for p in sorted(PROTO_DIR.glob('*.proto'))}

    problems: list[str] = []

    for name, spec_body in sorted(spec_files.items()):
        if name not in actual_files:
            problems.append(f'missing file: proto/edgerun/v0/{name}')
            continue
        actual_body = actual_files[name]

        spec_messages = parse_messages(spec_body)
        actual_messages = parse_messages(actual_body)
        for msg_name, spec_fields in sorted(spec_messages.items()):
            if msg_name not in actual_messages:
                problems.append(f'{name}: missing message {msg_name}')
                continue
            missing_fields = sorted(spec_fields - actual_messages[msg_name])
            for field in missing_fields:
                problems.append(f'{name}: message {msg_name} missing field `{field}`')

        spec_enums = parse_enums(spec_body)
        actual_enums = parse_enums(actual_body)
        for enum_name, spec_values in sorted(spec_enums.items()):
            if enum_name not in actual_enums:
                problems.append(f'{name}: missing enum {enum_name}')
                continue
            missing_values = sorted(spec_values - actual_enums[enum_name])
            for value in missing_values:
                problems.append(f'{name}: enum {enum_name} missing value `{value}`')

    spec_file_names = set(spec_files)
    extra_files = sorted(set(actual_files) - spec_file_names)

    if problems:
        print('PROTO/SPEC AUDIT FAILED')
        for problem in problems:
            print(f'- {problem}')
        return 1

    print('PROTO/SPEC AUDIT OK')
    print(f'- spec-defined files checked: {len(spec_files)}')
    print(f'- repo proto files present: {len(actual_files)}')
    if extra_files:
        print(f'- repo-local extension files: {", ".join(extra_files)}')
    else:
        print('- repo-local extension files: none')
    return 0


if __name__ == '__main__':
    sys.exit(main())
