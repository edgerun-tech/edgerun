// SPDX-License-Identifier: Apache-2.0
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use edgerun_storage::event::{ActorId, Event, StreamId};

fn bench_event_serialize(c: &mut Criterion) {
    let stream = StreamId::new();
    let actor = ActorId::new();
    let payload = vec![0xAB; 1024];
    let event = Event::new(stream, actor, payload);

    c.bench_function("event_serialize_1kb", |b| {
        b.iter(|| {
            let bytes = event.serialize().expect("serialize");
            black_box(bytes);
        })
    });
}

fn bench_event_deserialize(c: &mut Criterion) {
    let stream = StreamId::new();
    let actor = ActorId::new();
    let payload = vec![0xCD; 1024];
    let event = Event::new(stream, actor, payload);
    let serialized = event.serialize().expect("serialize");

    c.bench_function("event_deserialize_1kb", |b| {
        b.iter(|| {
            let evt = Event::deserialize(black_box(&serialized)).expect("deserialize");
            black_box(evt);
        })
    });
}

criterion_group!(
    event_benches,
    bench_event_serialize,
    bench_event_deserialize
);
criterion_main!(event_benches);
