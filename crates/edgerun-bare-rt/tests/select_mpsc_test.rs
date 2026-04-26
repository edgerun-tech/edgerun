use edgerun_bare_rt::{mpsc_channel, select_2};

#[test]
fn select_mpsc_with_runtime() {
    let rt = edgerun_bare_rt::Builder::new_multi_thread().build().unwrap();
    
    async fn demo() -> Option<i32> {
        let (tx, rx) = mpsc_channel();
        let mut recv_future = rx.recv();
        let mut other = Box::pin(async { Some(42) });
        
        tx.try_send(10).unwrap();
        
        select_2(&mut recv_future, other.as_mut()).await
    }
    
    let result = rt.block_on(Box::pin(demo()));
    assert!(result.is_some());
}