use edgerun_bare_rt::unbounded_channel;

#[test]
fn unbounded_channel_created() {
    let (sender, receiver) = unbounded_channel::<i32>();
    let _ = (sender, receiver);
}