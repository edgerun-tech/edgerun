// Shared WAT fragment parser — extracts function defs, call refs, global refs/defs
export function parseFragment(text) {
  const funcDefs = {};
  const calls = new Set();
  const globalRefs = new Set();
  const globalsDefs = new Set();
  const lines = text.split('\n');
  let inFunc = null, accType = '', depth = 0, inString = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    for (const m of line.matchAll(/call\s+\$(\w+)/g)) calls.add(m[1]);
    for (const m of line.matchAll(/global\.(?:get|set)\s+\$(\w+)/g)) globalRefs.add(m[1]);

    const gd = line.match(/^\s+\(global\s+\$(\w+)/);
    if (gd) globalsDefs.add(gd[1]);

    const defM = line.match(/^\s+\(func\s+\$?(\S+)/);
    if (defM) {
      const raw = defM[1];
      if (raw.startsWith('(')) { inFunc = '::anon' + i; continue; }
      inFunc = raw;
      accType = line.replace(/^\s+\(func\s+\$?\S+\s*/, '').trim();
      depth = 0; inString = false;
      for (const ch of line) {
        if (inString) { if (ch === '"') inString = false; }
        else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
      }
      if (depth === 0) {
        const hasExport = accType.includes('(export "') || line.includes('(export "');
        funcDefs[inFunc] = { type: accType, export: hasExport };
        inFunc = null;
      }
      continue;
    }

    if (inFunc) {
      for (const ch of line) {
        if (inString) { if (ch === '"') inString = false; }
        else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
      }
      if (depth === 0) {
        const hasExport = accType.includes('(export "') || line.includes('(export "');
        funcDefs[inFunc] = { type: accType, export: hasExport };
        inFunc = null;
      }
    }
  }
  return { funcDefs, calls, globalRefs, globalsDefs };
}
