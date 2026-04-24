use edgerun_rt::{Runtime, mpsc, spawn};
use std::time::Instant;

fn main() {
    let rt = Runtime::new_multi_thread().build().unwrap();
    rt.block_on(async {
        let start = Instant::now();
        
        let (tx, mut rx) = mpsc::channel::<u64>(1);
        let tx2 = tx.clone();
        
        let sender = spawn(async move {
            for i in 0..10u64 {
                eprintln!("[sender {}] calling send", i);
                let result = tx2.send(i).await;
                eprintln!("[sender {}] send returned {:?}", i, result);
            }
            eprintln!("[sender] DONE");
        });
        
        drop(tx);
        
        let mut count = 0;
        while count < 10 {
            eprintln!("[receiver {}] calling recv", count);
            match rx.recv().await {
                Some(v) => {
                    eprintln!("[receiver {}] got {}", count, v);
                    count += 1;
                }
                None => {
                    eprintln!("[receiver] closed");
                    break;
                }
            }
        }
        
        sender.await.ok();
        eprintln!("Got {}/10 in {:?}", count, start.elapsed());
    });
}