#!/usr/bin/env bun
// CLI ELF builder — delegates to tools/build.ts build-cli
// (formerly had its own build logic; consolidated into build.ts)

import { spawnSync } from 'child_process';
import { resolve } from 'path';

const args = process.argv.slice(2);
const tool = resolve(import.meta.dirname, '..', 'tools', 'build.ts');
const r = spawnSync('bun', [tool, 'build-cli', ...args], { stdio: 'inherit' });
process.exit(r.status ?? 1);
