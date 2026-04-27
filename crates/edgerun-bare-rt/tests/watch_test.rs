use edgerun_bare_rt::watch_channel;

#[test]
fn watch_channel_created() {
    let (sender, receiver) = watch_channel(42i32);
    let _ = (sender, receiver);
}
