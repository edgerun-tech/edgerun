use opentelemetry::trace::{SpanId, TraceId};
use std::fmt;

/// Interface for generating IDs
pub trait IdGenerator: Send + Sync + fmt::Debug {
    /// Generate a new `TraceId`
    fn new_trace_id(&self) -> TraceId;

    /// Generate a new `SpanId`
    fn new_span_id(&self) -> SpanId;
}

/// Default [`IdGenerator`] implementation.
///
/// Generates Trace and Span ids using a random number generator.
#[derive(Clone, Debug, Default)]
pub struct RandomIdGenerator {
    _private: (),
}

impl IdGenerator for RandomIdGenerator {
    fn new_trace_id(&self) -> TraceId {
        TraceId::from(edgerun_crypto::random_u128().expect("edgerun secure random source"))
    }

    fn new_span_id(&self) -> SpanId {
        SpanId::from(edgerun_crypto::random_u64().expect("edgerun secure random source"))
    }
}
