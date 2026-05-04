use anyhow::{bail, Context, Result};
use libc::{
    mmap, mprotect, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_EXEC, PROT_READ,
    PROT_WRITE,
};
use std::ptr;

use super::artifact::{DecodedAotArtifact, DecodedFunction};

pub struct LoadedFunction {
    ptr: *mut u8,
    len: usize,
    sig: ParsedSig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedSig {
    params: Vec<ScalarType>,
    result: Option<ScalarType>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarType {
    I32,
    I64,
}

impl LoadedFunction {
    pub fn from_artifact_function(func: &DecodedFunction) -> Result<Self> {
        let sig = ParsedSig::parse(&func.sig)
            .with_context(|| format!("unsupported AOT function signature for {}", func.name))?;
        if sig.params.len() > 6 {
            bail!("AOT loader only supports up to six integer arguments");
        }
        if func.code.is_empty() {
            bail!("AOT function {} has empty code", func.name);
        }

        let len = func.code.len();
        let ptr = unsafe {
            mmap(
                ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if ptr == MAP_FAILED {
            bail!("mmap failed for AOT function code");
        }

        unsafe {
            ptr::copy_nonoverlapping(func.code.as_ptr(), ptr as *mut u8, len);
        }

        let protect_result = unsafe { mprotect(ptr, len, PROT_READ | PROT_EXEC) };
        if protect_result != 0 {
            unsafe {
                let _ = munmap(ptr, len);
            }
            bail!("mprotect failed while sealing AOT function code executable");
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            len,
            sig,
        })
    }

    pub fn call_u64(&self, args: &[u64]) -> Result<u64> {
        if args.len() != self.sig.params.len() {
            bail!(
                "argument count mismatch: expected {}, got {}",
                self.sig.params.len(),
                args.len()
            );
        }

        let f: extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64 =
            unsafe { std::mem::transmute(self.ptr) };
        let mut a = [0u64; 6];
        a[..args.len()].copy_from_slice(args);
        let result = f(a[0], a[1], a[2], a[3], a[4], a[5]);

        Ok(match self.sig.result {
            Some(ScalarType::I32) => result & 0xffff_ffff,
            Some(ScalarType::I64) => result,
            None => 0,
        })
    }
}

impl Drop for LoadedFunction {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            unsafe {
                let _ = munmap(self.ptr as *mut _, self.len);
            }
        }
    }
}

impl ParsedSig {
    pub fn parse(sig: &str) -> Result<Self> {
        let (params, results) = sig
            .split_once(" -> ")
            .with_context(|| format!("invalid signature: {sig}"))?;
        let params = parse_tuple(params)?;
        let results = parse_tuple(results)?;
        if results.len() > 1 {
            bail!("AOT loader does not support multi-value returns");
        }
        Ok(Self {
            params,
            result: results.first().copied(),
        })
    }
}

pub fn find_function<'a>(
    artifact: &'a DecodedAotArtifact,
    selector: &str,
) -> Result<&'a DecodedFunction> {
    if let Ok(index) = selector.parse::<u32>() {
        return artifact
            .functions
            .iter()
            .find(|f| f.index == index)
            .with_context(|| format!("no AOT function index {index}"));
    }

    artifact
        .functions
        .iter()
        .find(|f| f.name == selector)
        .with_context(|| format!("no AOT function named {selector}"))
}

fn parse_tuple(s: &str) -> Result<Vec<ScalarType>> {
    let s = s.trim();
    if !s.starts_with('(') || !s.ends_with(')') {
        bail!("invalid tuple signature: {s}");
    }
    let inner = s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    inner
        .split(',')
        .map(|part| match part.trim() {
            "i32" => Ok(ScalarType::I32),
            "i64" => Ok(ScalarType::I64),
            other => bail!("unsupported scalar type in AOT signature: {other}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_signature() {
        let sig = ParsedSig::parse("() -> ()").unwrap();
        assert!(sig.params.is_empty());
        assert_eq!(sig.result, None);
    }

    #[test]
    fn parses_integer_signature() {
        let sig = ParsedSig::parse("(i32, i64) -> (i32)").unwrap();
        assert_eq!(sig.params, vec![ScalarType::I32, ScalarType::I64]);
        assert_eq!(sig.result, Some(ScalarType::I32));
    }
}
