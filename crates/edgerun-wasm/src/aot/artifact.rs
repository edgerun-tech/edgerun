use anyhow::{bail, Context, Result};

use super::ir::FunctionIr;

const MAGIC: &[u8; 8] = b"ERAOT001";
pub const ARTIFACT_VERSION: u32 = 2;

#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub index: u32,
    pub name: String,
    pub ir: FunctionIr,
    pub code: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct AotArtifact {
    pub wasm_sha256: [u8; 32],
    pub target: &'static str,
    pub compiler: &'static str,
    pub functions: Vec<CompiledFunction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedFunction {
    pub index: u32,
    pub name: String,
    pub sig: String,
    pub rendered_ir: String,
    pub code: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedAotArtifact {
    pub version: u32,
    pub target: String,
    pub compiler: String,
    pub wasm_sha256: [u8; 32],
    pub functions: Vec<DecodedFunction>,
}

impl AotArtifact {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        put_u32(&mut out, ARTIFACT_VERSION);
        put_bytes(&mut out, self.target.as_bytes());
        put_bytes(&mut out, self.compiler.as_bytes());
        out.extend_from_slice(&self.wasm_sha256);
        put_u32(&mut out, self.functions.len() as u32);

        for f in &self.functions {
            put_u32(&mut out, f.index);
            put_bytes(&mut out, f.name.as_bytes());
            put_bytes(&mut out, f.ir.sig.render().as_bytes());
            put_bytes(&mut out, f.ir.render().as_bytes());
            put_bytes(&mut out, &f.code);
        }

        out
    }
}

impl DecodedAotArtifact {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        let magic = cursor.take_exact(MAGIC.len()).context("missing AOT artifact magic")?;
        if magic != MAGIC.as_slice() {
            bail!("invalid AOT artifact magic");
        }

        let version = cursor.u32().context("missing AOT artifact version")?;
        if version != ARTIFACT_VERSION {
            bail!("unsupported AOT artifact version: {version}");
        }

        let target = cursor.string("target")?;
        let compiler = cursor.string("compiler")?;
        let wasm_hash = cursor.take_exact(32).context("missing WASM SHA-256")?;
        let mut wasm_sha256 = [0u8; 32];
        wasm_sha256.copy_from_slice(wasm_hash);

        let function_count = cursor.u32().context("missing function count")?;
        if function_count > 100_000 {
            bail!("unreasonable AOT function count: {function_count}");
        }

        let mut functions = Vec::with_capacity(function_count as usize);
        for _ in 0..function_count {
            let index = cursor.u32().context("missing function index")?;
            let name = cursor.string("function name")?;
            let sig = cursor.string("function signature")?;
            let rendered_ir = cursor.string("rendered IR")?;
            let code = cursor.bytes("machine code")?.to_vec();
            if code.is_empty() {
                bail!("compiled function {name} has empty machine code");
            }
            functions.push(DecodedFunction {
                index,
                name,
                sig,
                rendered_ir,
                code,
            });
        }

        if !cursor.is_empty() {
            bail!("trailing bytes after AOT artifact");
        }

        Ok(Self {
            version,
            target,
            compiler,
            wasm_sha256,
            functions,
        })
    }

    pub fn verify_for_wasm_sha256(&self, expected: &[u8; 32]) -> Result<()> {
        if &self.wasm_sha256 != expected {
            bail!("AOT artifact WASM hash does not match input WASM");
        }
        Ok(())
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn take_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .context("AOT artifact offset overflow")?;
        if end > self.bytes.len() {
            bail!(
                "truncated AOT artifact: need {} bytes at offset {}, len {}",
                len,
                self.offset,
                self.bytes.len()
            );
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32> {
        let bytes = self.take_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn bytes(&mut self, label: &str) -> Result<&'a [u8]> {
        let len = self.u32().with_context(|| format!("missing {label} length"))? as usize;
        self.take_exact(len)
            .with_context(|| format!("missing {label} payload"))
    }

    fn string(&mut self, label: &str) -> Result<String> {
        let bytes = self.bytes(label)?;
        std::str::from_utf8(bytes)
            .with_context(|| format!("{label} is not valid UTF-8"))
            .map(ToOwned::to_owned)
    }
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_has_stable_magic() {
        let artifact = AotArtifact {
            wasm_sha256: [7u8; 32],
            target: "x86_64-linux-sysv",
            compiler: "test",
            functions: Vec::new(),
        };
        assert!(artifact.encode().starts_with(MAGIC.as_slice()));
    }

    #[test]
    fn empty_function_artifact_round_trips() {
        let artifact = AotArtifact {
            wasm_sha256: [3u8; 32],
            target: "x86_64-linux-sysv",
            compiler: "test-compiler",
            functions: Vec::new(),
        };
        let decoded = DecodedAotArtifact::decode(&artifact.encode()).unwrap();
        assert_eq!(decoded.version, ARTIFACT_VERSION);
        assert_eq!(decoded.target, "x86_64-linux-sysv");
        assert_eq!(decoded.compiler, "test-compiler");
        assert_eq!(decoded.wasm_sha256, [3u8; 32]);
        assert!(decoded.functions.is_empty());
    }

    #[test]
    fn rejects_trailing_bytes() {
        let artifact = AotArtifact {
            wasm_sha256: [3u8; 32],
            target: "x86_64-linux-sysv",
            compiler: "test-compiler",
            functions: Vec::new(),
        };
        let mut bytes = artifact.encode();
        bytes.push(1);
        assert!(DecodedAotArtifact::decode(&bytes).is_err());
    }
}
