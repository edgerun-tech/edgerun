use edgerun_bare_rt::{channel, Sender};

#[test]
fn oneshot_await_receiver() {
    let rt = edgerun_bare_rt::Builder::new_multi_thread()
        .build()
        .unwrap();

    async fn demo() -> i32 {
        let (mut sender, receiver) = channel();
        sender.send(42).unwrap();
        receiver.await.unwrap()
    }

    let result = rt.block_on(Box::pin(demo()));
    assert_eq!(result, 42);
}
