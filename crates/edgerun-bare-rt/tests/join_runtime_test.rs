use edgerun_bare_rt::join_internal::join2;
use edgerun_bare_rt::Builder;

#[test]
fn join_with_runtime() {
    let rt = Builder::new_multi_thread().build().unwrap();

    async fn demo() -> (i32, i32) {
        join2(async { 10 }, async { 32 }).await
    }

    let result = rt.block_on(Box::pin(demo()));
    assert_eq!(result, (10, 32));
}
