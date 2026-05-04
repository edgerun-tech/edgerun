use crate::ir::FunctionIr;

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

impl AotArtifact {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"ERAOT001");
        put_u32(&mut out, 1);
        put_bytes(&mut out, self.target.as_bytes());
        put_bytes(&mut out, self.compiler.as_bytes());
        out.extend_from_slice(&self.wasm_sha256);
        put_u32(&mut out, self.functions.len() as u32);

        for f in &self.functions {
            put_u32(&mut out, f.index);
            put_bytes(&mut out, f.name.as_bytes());
            put_bytes(&mut out, f.ir.render().as_bytes());
            put_bytes(&mut out, &f.code);
        }

        out
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
        assert!(artifact.encode().starts_with(b"ERAOT001"));
    }
}
