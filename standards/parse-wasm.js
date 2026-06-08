import { readFileSync } from 'fs';

const buf = readFileSync('/tmp/cli-test.wasm');

const decoder = new TextDecoder();

let pos = 0;

function readByte() {
  return buf[pos++];
}

function readLEB128() {
  let result = 0;
  let shift = 0;
  while (true) {
    const byte = readByte();
    result |= (byte & 0x7f) << shift;
    if (!(byte & 0x80)) break;
    shift += 7;
  }
  return result;
}

function readLEB128_signed() {
  let result = 0;
  let shift = 0;
  let byte;
  while (true) {
    byte = readByte();
    result |= (byte & 0x7f) << shift;
    shift += 7;
    if (!(byte & 0x80)) break;
  }
  if (shift < 32 && (byte & 0x40)) {
    result |= - (1 << shift);
  }
  return result;
}

function readName() {
  const len = readLEB128();
  const str = decoder.decode(buf.slice(pos, pos + len));
  pos += len;
  return str;
}

function readVec(readFn) {
  const count = readLEB128();
  const items = [];
  for (let i = 0; i < count; i++) {
    items.push(readFn());
  }
  return items;
}

// Parse WASM header
const magic = buf.readUInt32LE(0);
const version = buf.readUInt32LE(4);
pos = 8;

console.log(`Magic: 0x${magic.toString(16)} (${decoder.decode(buf.slice(0,4))})`);
console.log(`Version: ${version}`);
console.log();

// Parse sections
let importCount = 0;
const imports = [];

while (pos < buf.length) {
  const sectionId = readByte();
  const sectionLen = readLEB128();
  const sectionStart = pos;
  const sectionEnd = pos + sectionLen;

  console.log(`\n=== Section ${sectionId} (${sectionLen} bytes) at offset 0x${(sectionStart-2).toString(16)}`);

  if (sectionId === 1) {
    // Type section
    const count = readLEB128();
    console.log(`Types count: ${count}`);
    for (let i = 0; i < count; i++) {
      const form = readByte();
      const paramCount = readLEB128();
      const params = [];
      for (let j = 0; j < paramCount; j++) {
        params.push(readByte());
      }
      const resultCount = readLEB128();
      const results = [];
      for (let j = 0; j < resultCount; j++) {
        results.push(readByte());
      }
      console.log(`  type[${i}]: params=[${params.join(',')}] results=[${results.join(',')}]`);
    }
  } else if (sectionId === 2) {
    // Import section
    importCount = readLEB128();
    console.log(`Import count: ${importCount}`);
    for (let i = 0; i < importCount; i++) {
      const mod = readName();
      const name = readName();
      const kind = readByte();
      let typeIdx;
      if (kind === 0) {
        typeIdx = readLEB128(); // func
      }
      imports.push({ mod, name, kind, typeIdx });
      console.log(`  import[${i}]: ${mod}.${name} (kind=${kind}, type=${typeIdx})`);
    }
  } else if (sectionId === 3) {
    // Function section
    const count = readLEB128();
    const funcTypes = [];
    for (let i = 0; i < count; i++) {
      funcTypes.push(readLEB128());
    }
    // Map local func index to type index (offset by import count)
    console.log(`Function count: ${count}`);
    console.log(`  Local functions start at index ${importCount}`);
    for (let i = 0; i < count; i++) {
      console.log(`  func[${i}] (global index ${importCount + i}): type=${funcTypes[i]}`);
    }
  } else if (sectionId === 4) {
    // Table section
    console.log(`Table section (skipping)`);
  } else if (sectionId === 5) {
    // Memory section
    console.log(`Memory section (skipping)`);
  } else if (sectionId === 6) {
    // Global section
    console.log(`Global section (skipping)`);
  } else if (sectionId === 7) {
    // Export section
    const count = readLEB128();
    console.log(`Export count: ${count}`);
    for (let i = 0; i < count; i++) {
      const name = readName();
      const kind = readByte();
      const idx = readLEB128();
      console.log(`  export: ${name} (kind=${kind}, index=${idx})`);
    }
  } else if (sectionId === 10) {
    // Code section
    const count = readLEB128();
    console.log(`Code count: ${count}`);
    // We need to map code entries to function indices
    // code[0] -> func[importCount + 0], etc.

    // For each code body, scan for call (0x10) and return_call (0x12)
    for (let funcIdx = 0; funcIdx < count; funcIdx++) {
      const bodySize = readLEB128();
      const bodyStart = pos;
      const bodyEnd = pos + bodySize;
      
      const localsCount = readLEB128();
      let totalLocals = 0;
      for (let l = 0; l < localsCount; l++) {
        const cnt = readLEB128();
        const valType = readByte();
        totalLocals += cnt;
      }

      const globalFuncIndex = importCount + funcIdx;
      console.log(`\n  Code body for func[${funcIdx}] (global index ${globalFuncIndex}) [${bodySize} bytes]:`);
      console.log(`    Locals: ${totalLocals}`);

      // Scan for call/return_call instructions in this function body
      let scanPos = pos;
      while (scanPos < bodyEnd) {
        const opcode = buf[scanPos];
        if (opcode === 0x10) {
          // call instruction: 0x10 followed by LEB128 func index
          const idx = readLEB128_at(scanPos + 1);
          const immSize = leb128_size(buf[scanPos + 1]);
          console.log(`    call ${idx} at offset 0x${scanPos.toString(16)}`);
          scanPos += 1 + immSize;
        } else if (opcode === 0x12) {
          // return_call instruction: 0x12 followed by LEB128 func index
          const idx = readLEB128_at(scanPos + 1);
          const immSize = leb128_size(buf[scanPos + 1]);
          console.log(`    return_call ${idx} at offset 0x${scanPos.toString(16)}`);
          scanPos += 1 + immSize;
        } else {
          scanPos++;
        }
      }

      pos = bodyEnd; // skip to next body
    }
  } else {
    console.log(`(skipping unknown section)`);
  }

  pos = sectionEnd;
}

// Re-scan the whole thing for accurate call/return_call counting
console.log('\n\n=== ACCURATE CALL SCAN ===');

const callCounts = {};
const returnCallCounts = {};

pos = 8; // skip header
while (pos < buf.length) {
  const sectionId = readByte();
  const sectionLen = readLEB128();
  const sectionStart = pos;
  const sectionEnd = pos + sectionLen;

  if (sectionId === 10) {
    const count = readLEB128();
    for (let funcIdx = 0; funcIdx < count; funcIdx++) {
      const bodySize = readLEB128();
      const bodyStart = pos;
      const bodyEnd = pos + bodySize;
      
      const localsCount = readLEB128();
      for (let l = 0; l < localsCount; l++) {
        const cnt = readLEB128();
        const valType = readByte();
      }

      let scanPos = pos;
      while (scanPos < bodyEnd) {
        const opcode = buf[scanPos];
        if (opcode === 0x10) {
          const idx = readLEB128_at(scanPos + 1);
          const immSize = leb128_size(buf[scanPos + 1]);
          callCounts[idx] = (callCounts[idx] || 0) + 1;
          scanPos += 1 + immSize;
        } else if (opcode === 0x12) {
          const idx = readLEB128_at(scanPos + 1);
          const immSize = leb128_size(buf[scanPos + 1]);
          returnCallCounts[idx] = (returnCallCounts[idx] || 0) + 1;
          scanPos += 1 + immSize;
        } else {
          scanPos++;
        }
      }
      pos = bodyEnd;
    }
  }
  pos = sectionEnd;
}

console.log('\nCall counts:');
const allCallIndices = new Set([...Object.keys(callCounts), ...Object.keys(returnCallCounts)]);
for (const idx of [...allCallIndices].sort((a,b) => a-b)) {
  const c = callCounts[idx] || 0;
  const r = returnCallCounts[idx] || 0;
  const label = idx < imports.length ? `(import: ${imports[idx].mod}.${imports[idx].name})` : `(local func ${idx})`;
  console.log(`  Index ${idx}: ${c} calls, ${r} return_calls ${label}`);
}

console.log('\nTotal call instructions:', Object.values(callCounts).reduce((a,b) => a+b, 0));
console.log('Total return_call instructions:', Object.values(returnCallCounts).reduce((a,b) => a+b, 0));

// Helper functions
function readLEB128_at(offset) {
  let result = 0;
  let shift = 0;
  let i = 0;
  while (true) {
    const byte = buf[offset + i];
    result |= (byte & 0x7f) << shift;
    shift += 7;
    i++;
    if (!(byte & 0x80)) break;
  }
  return result;
}

function leb128_size(firstByte) {
  let size = 1;
  let byte = firstByte;
  while (byte & 0x80) {
    size++;
  }
  return size;
}
