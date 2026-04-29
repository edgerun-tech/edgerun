use super::*;
use alloc::vec::Vec;

#[derive(Default)]
struct TestDigest {
    bytes: Vec<u8>,
}

impl LayerDigest for TestDigest {
    fn algorithm(&self) -> &'static str {
        "sha256"
    }

    fn update(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
    }

    fn finish(self) -> Vec<u8> {
        let mut out = [0u8; 32];
        for (index, byte) in self.bytes.iter().enumerate() {
            out[index % 32] = out[index % 32].wrapping_add(*byte);
        }
        out.to_vec()
    }
}

#[derive(Default)]
struct VecSink {
    bytes: Vec<u8>,
    finished: bool,
}

impl LayerSink for VecSink {
    fn write_chunk(&mut self, _descriptor: &LayerDescriptor, chunk: &[u8]) -> Result<(), String> {
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }

    fn finish_layer(&mut self, _descriptor: &LayerDescriptor) -> Result<(), String> {
        self.finished = true;
        Ok(())
    }
}

fn digest_for(bytes: &[u8]) -> String {
    let mut digest = TestDigest::default();
    digest.update(bytes);
    format!("sha256:{}", bytes_to_hex(&digest.finish()))
}

#[test]
fn streams_layer_chunks_into_sink() {
    let data = b"hello layer";
    let descriptor = LayerDescriptor {
        media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
        digest: digest_for(data),
        size: data.len() as u64,
    };
    let mut sink = VecSink::default();
    let chunks: [&[u8]; 2] = [&data[..5], &data[5..]];

    let report = apply_layer_chunks(&descriptor, chunks, TestDigest::default(), &mut sink).unwrap();

    assert_eq!(sink.bytes, data);
    assert!(sink.finished);
    assert_eq!(report.bytes_written, data.len() as u64);
    assert_eq!(report.digest, descriptor.digest);
}

#[test]
fn rejects_size_mismatch() {
    let descriptor = LayerDescriptor {
        media_type: None,
        digest: digest_for(b"abc"),
        size: 4,
    };
    let mut sink = VecSink::default();

    let error = apply_layer_chunks(
        &descriptor,
        [b"abc".as_slice()],
        TestDigest::default(),
        &mut sink,
    )
    .unwrap_err();

    assert_eq!(
        error,
        LayerPipelineError::SizeMismatch {
            expected: 4,
            actual: 3
        }
    );
}

#[test]
fn rejects_digest_mismatch() {
    let descriptor = LayerDescriptor {
        media_type: None,
        digest: digest_for(b"abc"),
        size: 3,
    };
    let mut sink = VecSink::default();

    let error = apply_layer_chunks(
        &descriptor,
        [b"abd".as_slice()],
        TestDigest::default(),
        &mut sink,
    )
    .unwrap_err();

    assert!(matches!(error, LayerPipelineError::DigestMismatch { .. }));
}
